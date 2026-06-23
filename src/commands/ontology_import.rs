//! OWL/RDF ontology importer — converts RDF/XML or JSON-LD to Fabric Ontology format.
//!
//! Compatible with the [Ontology Playground](https://github.com/microsoft/Ontology-Playground)
//! catalogue `.rdf` files. Parses `owl:Class`, `owl:DatatypeProperty`, and `owl:ObjectProperty`
//! into Fabric `EntityTypes` and `RelationshipTypes`.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::{ErrorCode, FabioError, enrich_forbidden};
use crate::output;

// ─── Public Entry Point ──────────────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
pub async fn import_owl(
    cli: &Cli,
    client: &FabricClient,
    workspace: Option<&str>,
    id: Option<&str>,
    file: &str,
    output_dir: Option<&str>,
) -> Result<()> {
    // Validate arguments
    if workspace.is_some() && id.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "--id is required when --workspace is specified.".to_string(),
            "Example: fabio ontology import --workspace <WS> --id <ID> --file ontology.rdf"
                .to_string(),
        )
        .into());
    }
    if workspace.is_none() && output_dir.is_none() {
        return Err(FabioError::with_hint(
            ErrorCode::InvalidInput,
            "Either --workspace (to push to Fabric) or --output-dir (to export locally) must be provided.".to_string(),
            "Example: fabio ontology import --file ontology.rdf --output-dir ./fabric-ontology/"
                .to_string(),
        )
        .into());
    }

    // Read and parse the file
    let content = fs::read_to_string(file)
        .map_err(|e| anyhow::anyhow!("Failed to read file '{file}': {e}"))?;

    let ext = Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Detect format: prefer file extension, fall back to content sniffing
    let format = match ext.as_str() {
        "rdf" | "owl" | "xml" => "rdf",
        "jsonld" | "json" => "jsonld",
        _ => {
            // No recognized extension — detect from content
            let trimmed = content.trim_start();
            if trimmed.starts_with('<') {
                "rdf"
            } else if trimmed.starts_with('{') || trimmed.starts_with('[') {
                "jsonld"
            } else {
                return Err(FabioError::with_hint(
                    ErrorCode::InvalidInput,
                    format!("Cannot detect format for file '{file}'"),
                    "Supported formats: .rdf, .owl, .xml (RDF/XML) or .jsonld, .json (JSON-LD). \
                     Alternatively, ensure the file content starts with '<' (XML) or '{{' (JSON)."
                        .to_string(),
                )
                .into());
            }
        }
    };

    let model = match format {
        "rdf" => parse_rdf_xml(&content),
        _ => parse_json_ld(&content)?,
    };

    // Convert to Fabric definition parts
    let parts = generate_fabric_parts(&model);

    if output::dry_run_guard(
        cli,
        "ontology import",
        &serde_json::json!({
            "file": file,
            "format": ext,
            "entity_types": model.classes.len(),
            "relationship_types": model.object_properties.len(),
            "total_properties": model.datatype_properties.len(),
        }),
    ) {
        return Ok(());
    }

    // Export to directory if requested
    if let Some(dir) = output_dir {
        write_to_directory(dir, &model, &parts)?;
    }

    // Push to Fabric if workspace+id provided
    if let (Some(ws), Some(ont_id)) = (workspace, id) {
        push_to_fabric(cli, client, ws, ont_id, &parts).await?;
    } else if output_dir.is_some() {
        // Only exported locally
        let obj = serde_json::json!({
            "status": "exported",
            "output_dir": output_dir,
            "entity_types": model.classes.len(),
            "relationship_types": model.object_properties.len(),
        });
        output::render_object(cli, &obj, "status");
    }

    Ok(())
}

// ─── Export (Fabric → OWL) ───────────────────────────────────────────────────

/// Fetch a Fabric Ontology definition and export it as OWL RDF/XML or JSON-LD.
pub async fn export_owl(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    format: &str,
    output_file: Option<&str>,
) -> Result<()> {
    let data = client
        .post(
            &format!("/workspaces/{workspace}/ontologies/{id}/getDefinition"),
            &serde_json::json!({}),
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "ontology export", "Contributor"))?;

    let model = fabric_definition_to_model(&data)?;
    let output_content = match format {
        "jsonld" => serialize_to_jsonld(&model),
        _ => serialize_to_rdf_xml(&model),
    };

    if let Some(path) = output_file {
        fs::write(path, &output_content)
            .map_err(|e| anyhow::anyhow!("Failed to write file '{path}': {e}"))?;
        let obj = serde_json::json!({
            "status": "exported",
            "file": path,
            "format": format,
            "entity_types": model.classes.len(),
            "relationship_types": model.object_properties.len(),
            "properties": model.datatype_properties.len(),
        });
        output::render_object(cli, &obj, "status");
    } else {
        print!("{output_content}");
    }
    Ok(())
}

fn fabric_definition_to_model(data: &Value) -> Result<OwlModel> {
    let parts = data
        .get("definition")
        .and_then(|d| d.get("parts"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("No definition parts found"))?;

    let mut model = OwlModel::default();
    let mut entity_id_to_uri: HashMap<String, String> = HashMap::new();

    for part in parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        if !path.contains("EntityTypes") || !path.ends_with("definition.json") {
            continue;
        }
        let payload = part.get("payload").and_then(Value::as_str).unwrap_or("");
        let decoded = BASE64.decode(payload).unwrap_or_default();
        let entity: Value =
            serde_json::from_str(&String::from_utf8_lossy(&decoded)).unwrap_or_default();

        let eid = entity.get("id").and_then(Value::as_str).unwrap_or("");
        let name = entity.get("name").and_then(Value::as_str).unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let uri = format!("http://fabric.microsoft.com/ontology/{name}");
        entity_id_to_uri.insert(eid.to_string(), uri.clone());
        model.classes.push(OwlClass {
            uri: uri.clone(),
            label: name.to_string(),
        });

        let id_parts: Vec<&str> = entity
            .get("entityIdParts")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |a| a.iter().filter_map(Value::as_str).collect());

        if let Some(props) = entity.get("properties").and_then(Value::as_array) {
            for prop in props {
                let pid = prop.get("id").and_then(Value::as_str).unwrap_or("");
                let pname = prop.get("name").and_then(Value::as_str).unwrap_or("");
                let vtype = prop
                    .get("valueType")
                    .and_then(Value::as_str)
                    .unwrap_or("String");
                model.datatype_properties.push(OwlDatatypeProperty {
                    label: pname.to_string(),
                    domain_uri: uri.clone(),
                    property_type: vtype.to_string(),
                    is_identifier: id_parts.contains(&pid),
                });
            }
        }
    }

    for part in parts {
        let path = part.get("path").and_then(Value::as_str).unwrap_or("");
        if !path.contains("RelationshipTypes") || !path.ends_with("definition.json") {
            continue;
        }
        let payload = part.get("payload").and_then(Value::as_str).unwrap_or("");
        let decoded = BASE64.decode(payload).unwrap_or_default();
        let rel: Value =
            serde_json::from_str(&String::from_utf8_lossy(&decoded)).unwrap_or_default();

        let name = rel.get("name").and_then(Value::as_str).unwrap_or("");
        let src = rel
            .get("source")
            .and_then(|s| s.get("entityTypeId"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let tgt = rel
            .get("target")
            .and_then(|t| t.get("entityTypeId"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let d = entity_id_to_uri.get(src).cloned().unwrap_or_default();
        let r = entity_id_to_uri.get(tgt).cloned().unwrap_or_default();
        if !d.is_empty() && !r.is_empty() {
            model.object_properties.push(OwlObjectProperty {
                label: name.to_string(),
                domain_uri: d,
                range_uri: r,
            });
        }
    }
    Ok(model)
}

#[allow(clippy::too_many_lines, clippy::write_with_newline)]
fn serialize_to_rdf_xml(model: &OwlModel) -> String {
    use std::fmt::Write;
    let base = "http://fabric.microsoft.com/ontology/";
    let mut s = String::new();
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rdf:RDF\n");
    let _ = write!(s, "    xml:base=\"{base}\"\n");
    s.push_str("    xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"\n");
    s.push_str("    xmlns:rdfs=\"http://www.w3.org/2000/01/rdf-schema#\"\n");
    s.push_str("    xmlns:owl=\"http://www.w3.org/2002/07/owl#\"\n");
    s.push_str("    xmlns:xsd=\"http://www.w3.org/2001/XMLSchema#\"\n");
    let _ = write!(s, "    xmlns:ont=\"{base}\">\n\n");
    for c in &model.classes {
        let _ = write!(
            s,
            "    <owl:Class rdf:about=\"{}\">\n        <rdfs:label>{}</rdfs:label>\n    </owl:Class>\n\n",
            c.uri, c.label
        );
    }
    for p in &model.datatype_properties {
        let xsd = fabric_type_to_xsd(&p.property_type);
        let _ = write!(
            s,
            "    <owl:DatatypeProperty rdf:about=\"{base}{}_{}\">",
            uri_local_name(&p.domain_uri).to_lowercase(),
            p.label
        );
        let _ = write!(s, "\n        <rdfs:label>{}</rdfs:label>", p.label);
        let _ = write!(
            s,
            "\n        <rdfs:domain rdf:resource=\"{}\"/>",
            p.domain_uri
        );
        let _ = write!(
            s,
            "\n        <rdfs:range rdf:resource=\"http://www.w3.org/2001/XMLSchema#{xsd}\"/>"
        );
        if p.is_identifier {
            s.push_str("\n        <ont:isIdentifier rdf:datatype=\"http://www.w3.org/2001/XMLSchema#boolean\">true</ont:isIdentifier>");
        }
        let _ = write!(
            s,
            "\n        <ont:propertyType>{}</ont:propertyType>",
            p.property_type.to_lowercase()
        );
        s.push_str("\n    </owl:DatatypeProperty>\n\n");
    }
    for r in &model.object_properties {
        let _ = write!(
            s,
            "    <owl:ObjectProperty rdf:about=\"{base}{}\">\n",
            r.label
        );
        let _ = write!(s, "        <rdfs:label>{}</rdfs:label>\n", r.label);
        let _ = write!(
            s,
            "        <rdfs:domain rdf:resource=\"{}\"/>\n",
            r.domain_uri
        );
        let _ = write!(
            s,
            "        <rdfs:range rdf:resource=\"{}\"/>\n",
            r.range_uri
        );
        s.push_str("    </owl:ObjectProperty>\n\n");
    }
    s.push_str("</rdf:RDF>\n");
    s
}

fn serialize_to_jsonld(model: &OwlModel) -> String {
    let mut graph: Vec<Value> = Vec::new();
    for c in &model.classes {
        graph.push(serde_json::json!({"@id": c.uri, "@type": "owl:Class", "rdfs:label": c.label}));
    }
    for p in &model.datatype_properties {
        let xsd = fabric_type_to_xsd(&p.property_type);
        let mut node = serde_json::json!({
            "@id": format!("{}#{}", p.domain_uri, p.label),
            "@type": "owl:DatatypeProperty",
            "rdfs:label": p.label,
            "rdfs:domain": {"@id": &p.domain_uri},
            "rdfs:range": {"@id": format!("http://www.w3.org/2001/XMLSchema#{xsd}")},
            "ont:propertyType": p.property_type.to_lowercase(),
        });
        if p.is_identifier {
            node["ont:isIdentifier"] = serde_json::json!(true);
        }
        graph.push(node);
    }
    for r in &model.object_properties {
        graph.push(serde_json::json!({
            "@id": format!("http://fabric.microsoft.com/ontology/{}", r.label),
            "@type": "owl:ObjectProperty",
            "rdfs:label": r.label,
            "rdfs:domain": {"@id": &r.domain_uri},
            "rdfs:range": {"@id": &r.range_uri},
        }));
    }
    let doc = serde_json::json!({
        "@context": {"owl": "http://www.w3.org/2002/07/owl#", "rdfs": "http://www.w3.org/2000/01/rdf-schema#", "xsd": "http://www.w3.org/2001/XMLSchema#", "ont": "http://fabric.microsoft.com/ontology/"},
        "@graph": graph
    });
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

fn fabric_type_to_xsd(t: &str) -> &str {
    match t {
        "BigInt" => "integer",
        "Double" => "decimal",
        "Boolean" => "boolean",
        "DateTime" => "dateTime",
        _ => "string",
    }
}

// ─── Public API for cross-module use ─────────────────────────────────────────

/// Public model struct for building OWL models externally (e.g., from context tenant).
pub struct OwlModelBuilder {
    pub classes: Vec<(String, String)>, // (uri, label)
    pub properties: Vec<(String, String, String, bool)>, // (label, domain_uri, type, is_id)
    pub relationships: Vec<(String, String, String)>, // (label, domain_uri, range_uri)
}

/// Serialize an externally-built OWL model to RDF/XML.
pub fn serialize_rdf_xml_from_model(builder: &OwlModelBuilder) -> String {
    let model = OwlModel {
        classes: builder
            .classes
            .iter()
            .map(|(uri, label)| OwlClass {
                uri: uri.clone(),
                label: label.clone(),
            })
            .collect(),
        datatype_properties: builder
            .properties
            .iter()
            .map(|(label, domain, ptype, is_id)| OwlDatatypeProperty {
                label: label.clone(),
                domain_uri: domain.clone(),
                property_type: ptype.clone(),
                is_identifier: *is_id,
            })
            .collect(),
        object_properties: builder
            .relationships
            .iter()
            .map(|(label, domain, range)| OwlObjectProperty {
                label: label.clone(),
                domain_uri: domain.clone(),
                range_uri: range.clone(),
            })
            .collect(),
    };
    serialize_to_rdf_xml(&model)
}

// ─── Data Model ──────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct OwlModel {
    classes: Vec<OwlClass>,
    datatype_properties: Vec<OwlDatatypeProperty>,
    object_properties: Vec<OwlObjectProperty>,
}

#[derive(Debug)]
struct OwlClass {
    uri: String,
    label: String,
}

#[derive(Debug)]
struct OwlDatatypeProperty {
    label: String,
    domain_uri: String,
    property_type: String,
    is_identifier: bool,
}

#[derive(Debug)]
struct OwlObjectProperty {
    label: String,
    domain_uri: String,
    range_uri: String,
}

// ─── RDF/XML Parser ──────────────────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
fn parse_rdf_xml(content: &str) -> OwlModel {
    let mut model = OwlModel::default();
    let mut reader = Reader::from_str(content);

    // State tracking for current element being parsed
    let mut in_class = false;
    let mut in_datatype_prop = false;
    let mut in_object_prop = false;
    let mut current_uri = String::new();
    let mut current_label = String::new();
    let mut current_domain = String::new();
    let mut current_range = String::new();
    let mut current_prop_type = String::new();
    let mut current_is_id = false;
    let mut reading_label = false;
    let mut reading_prop_type = false;
    let mut reading_is_id = false;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) | Err(_) => break,
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                let local_name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();

                #[allow(clippy::collapsible_match)]
                match local_name.as_str() {
                    "Class" => {
                        in_class = true;
                        current_uri = extract_rdf_about(e);
                        current_label.clear();
                    }
                    "DatatypeProperty" => {
                        in_datatype_prop = true;
                        current_uri = extract_rdf_about(e);
                        current_label.clear();
                        current_domain.clear();
                        current_range.clear();
                        current_prop_type.clear();
                        current_is_id = false;
                    }
                    "ObjectProperty" => {
                        in_object_prop = true;
                        current_uri = extract_rdf_about(e);
                        current_label.clear();
                        current_domain.clear();
                        current_range.clear();
                    }
                    "label" => reading_label = true,
                    "propertyType" => reading_prop_type = true,
                    "isIdentifier" => reading_is_id = true,
                    "domain" => {
                        if in_datatype_prop || in_object_prop {
                            current_domain = extract_rdf_resource(e);
                        }
                    }
                    "range" => {
                        if in_datatype_prop || in_object_prop {
                            current_range = extract_rdf_resource(e);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                if reading_label {
                    current_label = text;
                } else if reading_prop_type {
                    current_prop_type = text;
                } else if reading_is_id {
                    current_is_id = text == "true";
                }
            }
            Ok(Event::End(ref e)) => {
                let local_name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match local_name.as_str() {
                    "Class" => {
                        if in_class && !current_uri.is_empty() {
                            model.classes.push(OwlClass {
                                uri: current_uri.clone(),
                                label: if current_label.is_empty() {
                                    uri_local_name(&current_uri)
                                } else {
                                    current_label.clone()
                                },
                            });
                        }
                        in_class = false;
                    }
                    "DatatypeProperty" => {
                        if in_datatype_prop && !current_domain.is_empty() {
                            model.datatype_properties.push(OwlDatatypeProperty {
                                label: if current_label.is_empty() {
                                    uri_local_name(&current_uri)
                                } else {
                                    current_label.clone()
                                },
                                domain_uri: current_domain.clone(),
                                property_type: if current_prop_type.is_empty() {
                                    xsd_to_fabric_type(&current_range)
                                } else {
                                    playground_type_to_fabric(&current_prop_type)
                                },
                                is_identifier: current_is_id,
                            });
                        }
                        in_datatype_prop = false;
                    }
                    "ObjectProperty" => {
                        if in_object_prop && !current_domain.is_empty() && !current_range.is_empty()
                        {
                            model.object_properties.push(OwlObjectProperty {
                                label: if current_label.is_empty() {
                                    uri_local_name(&current_uri)
                                } else {
                                    current_label.clone()
                                },
                                domain_uri: current_domain.clone(),
                                range_uri: current_range.clone(),
                            });
                        }
                        in_object_prop = false;
                    }
                    "label" => reading_label = false,
                    "propertyType" => reading_prop_type = false,
                    "isIdentifier" => reading_is_id = false,
                    _ => {}
                }
            }
            _ => {}
        }
        buf.clear();
    }

    model
}

fn extract_rdf_about(e: &quick_xml::events::BytesStart<'_>) -> String {
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        if key.ends_with("about") || key == "rdf:about" {
            return String::from_utf8_lossy(&attr.value).to_string();
        }
    }
    String::new()
}

fn extract_rdf_resource(e: &quick_xml::events::BytesStart<'_>) -> String {
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        if key.ends_with("resource") || key == "rdf:resource" {
            return String::from_utf8_lossy(&attr.value).to_string();
        }
    }
    String::new()
}

fn uri_local_name(uri: &str) -> String {
    uri.rsplit_once('#')
        .or_else(|| uri.rsplit_once('/'))
        .map_or_else(|| uri.to_string(), |(_, name)| name.to_string())
}

fn xsd_to_fabric_type(xsd_uri: &str) -> String {
    let local = uri_local_name(xsd_uri).to_lowercase();
    match local.as_str() {
        "integer" | "int" | "long" => "BigInt",
        "decimal" | "double" | "float" => "Double",
        "boolean" | "bool" => "Boolean",
        "date" | "datetime" | "datetimestamp" => "DateTime",
        _ => "String",
    }
    .to_string()
}

fn playground_type_to_fabric(prop_type: &str) -> String {
    match prop_type.to_lowercase().as_str() {
        "integer" | "int" => "BigInt",
        "decimal" | "double" | "float" => "Double",
        "boolean" | "bool" => "Boolean",
        "date" | "datetime" => "DateTime",
        // "string", "enum", and everything else → String
        _ => "String",
    }
    .to_string()
}

// ─── JSON-LD Parser ──────────────────────────────────────────────────────────

fn parse_json_ld(content: &str) -> Result<OwlModel> {
    let root: Value = serde_json::from_str(content)?;

    // Handle {"data": {...}} envelope from fabio context tenant
    let data = root.get("data").unwrap_or(&root);

    let graph = data
        .get("@graph")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("JSON-LD must have an @graph array"))?;

    let mut model = OwlModel::default();

    for node in graph {
        let node_type = node.get("@type").and_then(Value::as_str).unwrap_or("");
        let node_id = node.get("@id").and_then(Value::as_str).unwrap_or("");
        let label = node
            .get("rdfs:label")
            .or_else(|| node.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        match node_type {
            "owl:Class" => {
                model.classes.push(OwlClass {
                    uri: node_id.to_string(),
                    label: if label.is_empty() {
                        uri_local_name(node_id)
                    } else {
                        label
                    },
                });
            }
            "owl:DatatypeProperty" => {
                let domain = node
                    .get("rdfs:domain")
                    .and_then(|d| d.get("@id").and_then(Value::as_str).or_else(|| d.as_str()))
                    .unwrap_or("")
                    .to_string();
                let range = node
                    .get("rdfs:range")
                    .and_then(|r| r.get("@id").and_then(Value::as_str).or_else(|| r.as_str()))
                    .unwrap_or("")
                    .to_string();
                let is_id = node
                    .get("ont:isIdentifier")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                if !domain.is_empty() {
                    model.datatype_properties.push(OwlDatatypeProperty {
                        label: if label.is_empty() {
                            uri_local_name(node_id)
                        } else {
                            label
                        },
                        domain_uri: domain,
                        property_type: xsd_to_fabric_type(&range),
                        is_identifier: is_id,
                    });
                }
            }
            "owl:ObjectProperty" => {
                let domain = node
                    .get("rdfs:domain")
                    .and_then(|d| d.get("@id").and_then(Value::as_str).or_else(|| d.as_str()))
                    .unwrap_or("")
                    .to_string();
                let range = node
                    .get("rdfs:range")
                    .and_then(|r| r.get("@id").and_then(Value::as_str).or_else(|| r.as_str()))
                    .unwrap_or("")
                    .to_string();

                if !domain.is_empty() && !range.is_empty() {
                    model.object_properties.push(OwlObjectProperty {
                        label: if label.is_empty() {
                            uri_local_name(node_id)
                        } else {
                            label
                        },
                        domain_uri: domain,
                        range_uri: range,
                    });
                }
            }
            _ => {
                // For non-standard JSON-LD (like fabio context tenant output),
                // treat typed nodes as classes
                if !node_type.is_empty() && node_type != "fabric:Workspace" {
                    let clean_type = node_type.replace("fabric:", "");
                    // Only add the type if we haven't seen it
                    if !model.classes.iter().any(|c| c.label == clean_type) {
                        model.classes.push(OwlClass {
                            uri: format!("urn:fabric:type:{clean_type}"),
                            label: clean_type,
                        });
                    }
                }
            }
        }
    }

    Ok(model)
}

// ─── Fabric Format Generator ─────────────────────────────────────────────────

struct FabricPart {
    path: String,
    content: String,
}

fn generate_fabric_parts(model: &OwlModel) -> Vec<FabricPart> {
    let mut parts = Vec::new();

    // Root definition.json
    parts.push(FabricPart {
        path: "definition.json".to_string(),
        content: "{}".to_string(),
    });

    // Build class URI → ID mapping
    let mut class_ids: HashMap<String, String> = HashMap::new();
    for (i, class) in model.classes.iter().enumerate() {
        let id = format!("888{:010}", i + 1);
        class_ids.insert(class.uri.clone(), id);
    }

    // Generate EntityTypes
    for class in &model.classes {
        let type_id = class_ids.get(&class.uri).unwrap();

        // Collect properties for this class
        let mut properties: Vec<Value> = Vec::new();
        let mut id_parts: Vec<String> = Vec::new();
        let mut display_name_id: Option<String> = None;

        for (pi, prop) in model
            .datatype_properties
            .iter()
            .filter(|p| p.domain_uri == class.uri)
            .enumerate()
        {
            let prop_id = format!("{type_id}{:02}", pi + 1);
            properties.push(serde_json::json!({
                "id": prop_id,
                "name": prop.label,
                "valueType": prop.property_type,
            }));

            if prop.is_identifier {
                id_parts.push(prop_id.clone());
            }
            // Use first string property as display name if no identifier found
            if display_name_id.is_none() && prop.property_type == "String" {
                display_name_id = Some(prop_id.clone());
            }
        }

        // If no identifier was marked, use first property
        if id_parts.is_empty()
            && let Some(first) = properties.first()
            && let Some(pid) = first.get("id").and_then(Value::as_str)
        {
            id_parts.push(pid.to_string());
        }

        let entity_def = serde_json::json!({
            "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/ontology/entityType/1.0.0/schema.json",
            "id": type_id,
            "namespace": "usertypes",
            "name": class.label,
            "namespaceType": "Custom",
            "visibility": "Visible",
            "displayNamePropertyId": display_name_id.as_deref().unwrap_or(""),
            "entityIdParts": id_parts,
            "properties": properties,
        });

        parts.push(FabricPart {
            path: format!("EntityTypes/{type_id}/definition.json"),
            content: serde_json::to_string_pretty(&entity_def).unwrap_or_default(),
        });
    }

    // Generate RelationshipTypes
    for (i, rel) in model.object_properties.iter().enumerate() {
        let rel_id = format!("999{:010}", i + 1);

        let source_id = class_ids.get(&rel.domain_uri).cloned().unwrap_or_default();
        let target_id = class_ids.get(&rel.range_uri).cloned().unwrap_or_default();

        if source_id.is_empty() || target_id.is_empty() {
            continue; // Skip if source or target class not found
        }

        let rel_def = serde_json::json!({
            "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/ontology/relationshipType/1.0.0/schema.json",
            "id": rel_id,
            "namespace": "usertypes",
            "name": rel.label,
            "namespaceType": "Custom",
            "source": {"entityTypeId": source_id},
            "target": {"entityTypeId": target_id},
        });

        parts.push(FabricPart {
            path: format!("RelationshipTypes/{rel_id}/definition.json"),
            content: serde_json::to_string_pretty(&rel_def).unwrap_or_default(),
        });
    }

    parts
}

// ─── Directory Export ────────────────────────────────────────────────────────

fn write_to_directory(dir: &str, model: &OwlModel, parts: &[FabricPart]) -> Result<()> {
    for part in parts {
        let full_path = Path::new(dir).join(&part.path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full_path, &part.content)?;
    }

    eprintln!(
        "[ontology import] Exported {} entity types, {} relationship types to {dir}",
        model.classes.len(),
        model.object_properties.len()
    );
    Ok(())
}

// ─── Fabric API Push ─────────────────────────────────────────────────────────

async fn push_to_fabric(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    parts: &[FabricPart],
) -> Result<()> {
    // Build definition parts array
    let api_parts: Vec<Value> = parts
        .iter()
        .map(|p| {
            serde_json::json!({
                "path": p.path,
                "payload": BASE64.encode(p.content.as_bytes()),
                "payloadType": "InlineBase64"
            })
        })
        .collect();

    let body = serde_json::json!({
        "definition": {
            "parts": api_parts
        }
    });

    let data = client
        .post(
            &format!("/workspaces/{workspace}/ontologies/{id}/updateDefinition"),
            &body,
            true,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "ontology import", "Contributor"))?;

    let entity_count = parts
        .iter()
        .filter(|p| p.path.contains("EntityTypes"))
        .count();
    let rel_count = parts
        .iter()
        .filter(|p| p.path.contains("RelationshipTypes"))
        .count();

    if data.is_null() || data.as_object().is_some_and(serde_json::Map::is_empty) {
        let obj = serde_json::json!({
            "status": "imported",
            "id": id,
            "entity_types": entity_count,
            "relationship_types": rel_count,
        });
        output::render_object(cli, &obj, "status");
    } else {
        output::render_object(cli, &data, "id");
    }
    Ok(())
}

// ─── Unit Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rdf_xml_classes() {
        let rdf = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
         xmlns:owl="http://www.w3.org/2002/07/owl#">
  <owl:Class rdf:about="http://example.org/Customer">
    <rdfs:label>Customer</rdfs:label>
  </owl:Class>
  <owl:Class rdf:about="http://example.org/Order">
    <rdfs:label>Order</rdfs:label>
  </owl:Class>
</rdf:RDF>"#;
        let model = parse_rdf_xml(rdf);
        assert_eq!(model.classes.len(), 2);
        assert_eq!(model.classes[0].label, "Customer");
        assert_eq!(model.classes[1].label, "Order");
    }

    #[test]
    fn test_parse_rdf_xml_properties_with_types() {
        let rdf = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
         xmlns:owl="http://www.w3.org/2002/07/owl#"
         xmlns:xsd="http://www.w3.org/2001/XMLSchema#"
         xmlns:ont="http://example.org/">
  <owl:Class rdf:about="http://example.org/Product">
    <rdfs:label>Product</rdfs:label>
  </owl:Class>
  <owl:DatatypeProperty rdf:about="http://example.org/product_price">
    <rdfs:label>price</rdfs:label>
    <rdfs:domain rdf:resource="http://example.org/Product"/>
    <rdfs:range rdf:resource="http://www.w3.org/2001/XMLSchema#decimal"/>
    <ont:propertyType>decimal</ont:propertyType>
  </owl:DatatypeProperty>
  <owl:DatatypeProperty rdf:about="http://example.org/product_id">
    <rdfs:label>productId</rdfs:label>
    <rdfs:domain rdf:resource="http://example.org/Product"/>
    <rdfs:range rdf:resource="http://www.w3.org/2001/XMLSchema#string"/>
    <ont:isIdentifier rdf:datatype="http://www.w3.org/2001/XMLSchema#boolean">true</ont:isIdentifier>
  </owl:DatatypeProperty>
</rdf:RDF>"#;
        let model = parse_rdf_xml(rdf);
        assert_eq!(model.classes.len(), 1);
        assert_eq!(model.datatype_properties.len(), 2);

        let price = &model.datatype_properties[0];
        assert_eq!(price.label, "price");
        assert_eq!(price.property_type, "Double");
        assert!(!price.is_identifier);

        let pid = &model.datatype_properties[1];
        assert_eq!(pid.label, "productId");
        assert!(pid.is_identifier);
    }

    #[test]
    fn test_parse_rdf_xml_relationships() {
        let rdf = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
         xmlns:owl="http://www.w3.org/2002/07/owl#">
  <owl:Class rdf:about="http://example.org/Customer"><rdfs:label>Customer</rdfs:label></owl:Class>
  <owl:Class rdf:about="http://example.org/Order"><rdfs:label>Order</rdfs:label></owl:Class>
  <owl:ObjectProperty rdf:about="http://example.org/places">
    <rdfs:label>places</rdfs:label>
    <rdfs:domain rdf:resource="http://example.org/Customer"/>
    <rdfs:range rdf:resource="http://example.org/Order"/>
  </owl:ObjectProperty>
</rdf:RDF>"#;
        let model = parse_rdf_xml(rdf);
        assert_eq!(model.object_properties.len(), 1);
        assert_eq!(model.object_properties[0].label, "places");
        assert_eq!(
            model.object_properties[0].domain_uri,
            "http://example.org/Customer"
        );
        assert_eq!(
            model.object_properties[0].range_uri,
            "http://example.org/Order"
        );
    }

    #[test]
    fn test_parse_json_ld_owl_classes() {
        let jsonld = r#"{
            "@graph": [
                {"@id": "http://ex.org/Cat", "@type": "owl:Class", "rdfs:label": "Category"},
                {"@id": "http://ex.org/Item", "@type": "owl:Class", "rdfs:label": "Item"}
            ]
        }"#;
        let model = parse_json_ld(jsonld).unwrap();
        assert_eq!(model.classes.len(), 2);
        assert_eq!(model.classes[0].label, "Category");
        assert_eq!(model.classes[1].label, "Item");
    }

    #[test]
    fn test_parse_json_ld_fabric_context_output() {
        let jsonld = r#"{"data": {"@context": {}, "@graph": [
            {"@id": "urn:fabric:item:abc", "@type": "fabric:Notebook", "name": "ETL"},
            {"@id": "urn:fabric:item:def", "@type": "fabric:Lakehouse", "name": "Sales"},
            {"@id": "urn:fabric:workspace:ws1", "@type": "fabric:Workspace", "name": "Demo"}
        ]}}"#;
        let model = parse_json_ld(jsonld).unwrap();
        // Workspaces are excluded, unique types extracted
        assert_eq!(model.classes.len(), 2);
        let names: Vec<&str> = model.classes.iter().map(|c| c.label.as_str()).collect();
        assert!(names.contains(&"Notebook"));
        assert!(names.contains(&"Lakehouse"));
    }

    #[test]
    fn test_xsd_type_mapping() {
        assert_eq!(
            xsd_to_fabric_type("http://www.w3.org/2001/XMLSchema#string"),
            "String"
        );
        assert_eq!(
            xsd_to_fabric_type("http://www.w3.org/2001/XMLSchema#integer"),
            "BigInt"
        );
        assert_eq!(
            xsd_to_fabric_type("http://www.w3.org/2001/XMLSchema#decimal"),
            "Double"
        );
        assert_eq!(
            xsd_to_fabric_type("http://www.w3.org/2001/XMLSchema#boolean"),
            "Boolean"
        );
        assert_eq!(
            xsd_to_fabric_type("http://www.w3.org/2001/XMLSchema#dateTime"),
            "DateTime"
        );
        assert_eq!(xsd_to_fabric_type("http://example.org/unknown"), "String");
    }

    #[test]
    fn test_playground_type_mapping() {
        assert_eq!(playground_type_to_fabric("string"), "String");
        assert_eq!(playground_type_to_fabric("enum"), "String");
        assert_eq!(playground_type_to_fabric("integer"), "BigInt");
        assert_eq!(playground_type_to_fabric("decimal"), "Double");
        assert_eq!(playground_type_to_fabric("boolean"), "Boolean");
        assert_eq!(playground_type_to_fabric("datetime"), "DateTime");
        assert_eq!(playground_type_to_fabric("date"), "DateTime");
    }

    #[test]
    fn test_generate_fabric_parts() {
        let model = OwlModel {
            classes: vec![
                OwlClass {
                    uri: "http://ex.org/A".to_string(),
                    label: "TypeA".to_string(),
                },
                OwlClass {
                    uri: "http://ex.org/B".to_string(),
                    label: "TypeB".to_string(),
                },
            ],
            datatype_properties: vec![OwlDatatypeProperty {
                label: "name".to_string(),
                domain_uri: "http://ex.org/A".to_string(),
                property_type: "String".to_string(),
                is_identifier: true,
            }],
            object_properties: vec![OwlObjectProperty {
                label: "relatesTo".to_string(),
                domain_uri: "http://ex.org/A".to_string(),
                range_uri: "http://ex.org/B".to_string(),
            }],
        };
        let parts = generate_fabric_parts(&model);
        // root + 2 entities + 1 relationship = 4 parts
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0].path, "definition.json");
        assert!(parts[1].path.contains("EntityTypes"));
        assert!(parts[2].path.contains("EntityTypes"));
        assert!(parts[3].path.contains("RelationshipTypes"));

        // Verify entity content
        let entity: serde_json::Value = serde_json::from_str(&parts[1].content).unwrap();
        assert_eq!(entity["name"], "TypeA");
        assert_eq!(entity["properties"][0]["name"], "name");
        assert_eq!(entity["properties"][0]["valueType"], "String");
    }

    #[test]
    fn test_uri_local_name() {
        assert_eq!(uri_local_name("http://example.org/foo/Bar"), "Bar");
        assert_eq!(uri_local_name("http://example.org#Baz"), "Baz");
        assert_eq!(uri_local_name("JustAName"), "JustAName");
    }

    #[test]
    fn test_content_detection_xml() {
        let xml_content = "<?xml version=\"1.0\"?>\n<rdf:RDF>...</rdf:RDF>";
        assert!(xml_content.trim_start().starts_with('<'));
    }

    #[test]
    fn test_content_detection_json() {
        let json_content = "{\"@graph\": []}";
        let trimmed = json_content.trim_start();
        assert!(trimmed.starts_with('{') || trimmed.starts_with('['));
    }

    #[test]
    fn test_serialize_to_rdf_xml() {
        let model = OwlModel {
            classes: vec![OwlClass {
                uri: "http://ex.org/Thing".to_string(),
                label: "Thing".to_string(),
            }],
            datatype_properties: vec![OwlDatatypeProperty {
                label: "name".to_string(),
                domain_uri: "http://ex.org/Thing".to_string(),
                property_type: "String".to_string(),
                is_identifier: true,
            }],
            object_properties: vec![],
        };
        let rdf = serialize_to_rdf_xml(&model);
        assert!(rdf.contains("owl:Class"));
        assert!(rdf.contains("Thing"));
        assert!(rdf.contains("owl:DatatypeProperty"));
        assert!(rdf.contains("ont:isIdentifier"));
        assert!(rdf.contains("XMLSchema#string"));
    }

    #[test]
    fn test_serialize_to_jsonld() {
        let model = OwlModel {
            classes: vec![
                OwlClass {
                    uri: "http://ex.org/A".to_string(),
                    label: "A".to_string(),
                },
                OwlClass {
                    uri: "http://ex.org/B".to_string(),
                    label: "B".to_string(),
                },
            ],
            datatype_properties: vec![OwlDatatypeProperty {
                label: "score".to_string(),
                domain_uri: "http://ex.org/A".to_string(),
                property_type: "Double".to_string(),
                is_identifier: false,
            }],
            object_properties: vec![OwlObjectProperty {
                label: "links".to_string(),
                domain_uri: "http://ex.org/A".to_string(),
                range_uri: "http://ex.org/B".to_string(),
            }],
        };
        let jsonld = serialize_to_jsonld(&model);
        let doc: serde_json::Value = serde_json::from_str(&jsonld).unwrap();
        assert!(doc.get("@context").is_some());
        let graph = doc["@graph"].as_array().unwrap();
        // 2 classes + 1 property + 1 relationship = 4 nodes
        assert_eq!(graph.len(), 4);
        assert_eq!(
            graph.iter().filter(|n| n["@type"] == "owl:Class").count(),
            2
        );
        let rels: Vec<_> = graph
            .iter()
            .filter(|n| n["@type"] == "owl:ObjectProperty")
            .collect();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0]["rdfs:label"], "links");
    }

    #[test]
    fn test_fabric_type_to_xsd() {
        assert_eq!(fabric_type_to_xsd("String"), "string");
        assert_eq!(fabric_type_to_xsd("BigInt"), "integer");
        assert_eq!(fabric_type_to_xsd("Double"), "decimal");
        assert_eq!(fabric_type_to_xsd("Boolean"), "boolean");
        assert_eq!(fabric_type_to_xsd("DateTime"), "dateTime");
        assert_eq!(fabric_type_to_xsd("Unknown"), "string");
    }
}
