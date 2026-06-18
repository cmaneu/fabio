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
    uri.rsplit_once('/')
        .or_else(|| uri.rsplit_once('#'))
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
        if id_parts.is_empty() {
            if let Some(first) = properties.first() {
                if let Some(pid) = first.get("id").and_then(Value::as_str) {
                    id_parts.push(pid.to_string());
                }
            }
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
