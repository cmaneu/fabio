use std::io::Read;
use std::path::Path;

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use clap::Subcommand;
use serde_json::Value;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::errors::enrich_forbidden;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum OntologyCommand {
    /// List ontologies in a workspace
    List {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,
    },
    /// Show details of an ontology
    Show {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Ontology ID
        #[arg(long)]
        id: String,
    },
    /// Create an ontology
    Create {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Display name (must start with letter, alphanumeric/underscore, <100 chars)
        #[arg(long)]
        name: String,

        /// Description (max 256 characters)
        #[arg(long)]
        description: Option<String>,

        /// Path to definition JSON file (base64-encoded parts format)
        #[arg(long, conflicts_with = "file")]
        definition: Option<String>,

        /// Path to a local RDF file (.ttl, .owl, .rdf, .jsonld, .nt, .n3, .trig)
        /// Auto-detects format from extension and wraps into Fabric definition
        #[arg(long, conflicts_with = "definition")]
        file: Option<String>,
    },
    /// Update ontology properties (name and/or description)
    Update {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Ontology ID
        #[arg(long)]
        id: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete an ontology
    Delete {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Ontology ID
        #[arg(long)]
        id: String,

        /// Permanently delete (cannot be recovered)
        #[arg(long)]
        hard: bool,
    },
    /// Get the ontology definition (entity types, bindings)
    GetDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Ontology ID
        #[arg(long)]
        id: String,

        /// Definition format
        #[arg(long)]
        format: Option<String>,
    },
    /// Update the ontology definition (replaces current definition)
    UpdateDefinition {
        /// Workspace ID
        #[arg(short, long)]
        workspace: String,

        /// Ontology ID
        #[arg(long)]
        id: String,

        /// Path to definition JSON file, or - for stdin
        #[arg(long, conflicts_with = "file")]
        definition: Option<String>,

        /// Path to a local RDF file (.ttl, .owl, .rdf, .jsonld, .nt, .n3, .trig)
        /// Auto-detects format from extension and wraps into Fabric definition
        #[arg(long, conflicts_with = "definition")]
        file: Option<String>,

        /// Also update item metadata from .platform file
        #[arg(long)]
        update_metadata: bool,
    },
}

pub async fn execute(cli: &Cli, client: &FabricClient, command: &OntologyCommand) -> Result<()> {
    match command {
        OntologyCommand::List { workspace } => list(cli, client, workspace).await,
        OntologyCommand::Show { workspace, id } => show(cli, client, workspace, id).await,
        OntologyCommand::Create {
            workspace,
            name,
            description,
            definition,
            file,
        } => create(
            cli,
            client,
            workspace,
            name,
            description.as_deref(),
            definition.as_deref(),
            file.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "ontology create", "Member")),
        OntologyCommand::Update {
            workspace,
            id,
            name,
            description,
        } => update(
            cli,
            client,
            workspace,
            id,
            name.as_deref(),
            description.as_deref(),
        )
        .await
        .map_err(|e| enrich_forbidden(e, "ontology update", "Contributor")),
        OntologyCommand::Delete {
            workspace,
            id,
            hard,
        } => delete(cli, client, workspace, id, *hard)
            .await
            .map_err(|e| enrich_forbidden(e, "ontology delete", "Member")),
        OntologyCommand::GetDefinition {
            workspace,
            id,
            format,
        } => get_definition(cli, client, workspace, id, format.as_deref()).await,
        OntologyCommand::UpdateDefinition {
            workspace,
            id,
            definition,
            file,
            update_metadata,
        } => update_definition(
            cli,
            client,
            workspace,
            id,
            definition.as_deref(),
            file.as_deref(),
            *update_metadata,
        )
        .await
        .map_err(|e| enrich_forbidden(e, "ontology update-definition", "Contributor")),
    }
}

async fn list(cli: &Cli, client: &FabricClient, workspace: &str) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/workspaces/{workspace}/ontologies"),
            "value",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["displayName", "id", "description"],
        &["NAME", "ID", "DESCRIPTION"],
        "displayName",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}

async fn show(cli: &Cli, client: &FabricClient, workspace: &str, id: &str) -> Result<()> {
    let data = client
        .get(&format!("/workspaces/{workspace}/ontologies/{id}"))
        .await?;

    output::render_object(cli, &data, "id");
    Ok(())
}

async fn create(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    name: &str,
    description: Option<&str>,
    definition_path: Option<&str>,
    file_path: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::json!({
        "displayName": name,
    });

    if let Some(desc) = description {
        body["description"] = Value::String(desc.to_string());
    }

    if let Some(path) = definition_path {
        let content = read_file_or_stdin(path)?;
        let def: Value = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Invalid definition JSON: {e}"))?;
        body["definition"] = def;
    } else if let Some(path) = file_path {
        body["definition"] = build_definition_from_rdf(path)?;
    }

    let data = client
        .post(&format!("/workspaces/{workspace}/ontologies"), &body, true)
        .await?;

    output::render_object(cli, &data, "id");
    Ok(())
}

async fn update(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if name.is_none() && description.is_none() {
        anyhow::bail!("Specify at least one of --name or --description to update");
    }

    let mut body = serde_json::json!({});
    if let Some(n) = name {
        body["displayName"] = Value::String(n.to_string());
    }
    if let Some(d) = description {
        body["description"] = Value::String(d.to_string());
    }

    let data = client
        .patch(&format!("/workspaces/{workspace}/ontologies/{id}"), &body)
        .await?;

    output::render_object(cli, &data, "id");
    Ok(())
}

async fn delete(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    hard: bool,
) -> Result<()> {
    let path = if hard {
        format!("/workspaces/{workspace}/ontologies/{id}?hardDelete=True")
    } else {
        format!("/workspaces/{workspace}/ontologies/{id}")
    };

    client.delete(&path).await?;

    output::render_object(
        cli,
        &serde_json::json!({"id": id, "status": "deleted"}),
        "status",
    );
    Ok(())
}

async fn get_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    format: Option<&str>,
) -> Result<()> {
    let path = format.map_or_else(
        || format!("/workspaces/{workspace}/ontologies/{id}/getDefinition"),
        |f| format!("/workspaces/{workspace}/ontologies/{id}/getDefinition?format={f}"),
    );

    let data = client.post(&path, &serde_json::json!({}), true).await?;

    output::render_object(cli, &data, "definition");
    Ok(())
}

async fn update_definition(
    cli: &Cli,
    client: &FabricClient,
    workspace: &str,
    id: &str,
    definition_path: Option<&str>,
    file_path: Option<&str>,
    update_metadata: bool,
) -> Result<()> {
    let def = if let Some(path) = definition_path {
        let content = read_file_or_stdin(path)?;
        serde_json::from_str::<Value>(&content)
            .map_err(|e| anyhow::anyhow!("Invalid definition JSON: {e}"))?
    } else if let Some(path) = file_path {
        build_definition_from_rdf(path)?
    } else {
        anyhow::bail!("Specify either --definition or --file");
    };

    let body = serde_json::json!({"definition": def});

    let path = if update_metadata {
        format!("/workspaces/{workspace}/ontologies/{id}/updateDefinition?updateMetadata=True")
    } else {
        format!("/workspaces/{workspace}/ontologies/{id}/updateDefinition")
    };

    let data = client.post(&path, &body, true).await?;

    output::render_object(cli, &data, "status");
    Ok(())
}

/// Build a Fabric definition payload from a raw RDF file.
/// Auto-detects format from file extension and wraps content as base64-encoded part.
/// Includes the mandatory `definition.json` part that Fabric requires.
fn build_definition_from_rdf(path: &str) -> Result<Value> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let part_path = match ext.as_str() {
        "ttl" => "ontology.ttl",
        "owl" => "ontology.owl",
        "rdf" | "xml" => "ontology.rdf",
        "jsonld" => "ontology.jsonld",
        "nt" => "ontology.nt",
        "n3" => "ontology.n3",
        "trig" => "ontology.trig",
        _ => anyhow::bail!(
            "Unsupported RDF format '.{ext}'. Supported: .ttl, .owl, .rdf, .xml, .jsonld, .nt, .n3, .trig"
        ),
    };

    let content = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("Failed to read RDF file '{path}': {e}"))?;

    let encoded = BASE64.encode(&content);

    // Fabric requires a definition.json part to exist; include it as empty JSON
    let def_json_payload = BASE64.encode(b"{}");

    Ok(serde_json::json!({
        "parts": [
            {
                "path": "definition.json",
                "payload": def_json_payload,
                "payloadType": "InlineBase64"
            },
            {
                "path": part_path,
                "payload": encoded,
                "payloadType": "InlineBase64"
            }
        ]
    }))
}

fn read_file_or_stdin(path: &str) -> Result<String> {
    if path == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| anyhow::anyhow!("Failed to read from stdin: {e}"))?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{path}': {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_definition_from_rdf_ttl() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("schema.ttl");
        std::fs::write(
            &file,
            r#"@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix sales: <http://example.org/sales#> .

sales:SalesOntology a owl:Ontology ;
    rdfs:label "Sales Domain Ontology" .

sales:Customer a owl:Class ;
    rdfs:label "Customer" .

sales:Order a owl:Class ;
    rdfs:label "Order" .

sales:placedBy a owl:ObjectProperty ;
    rdfs:domain sales:Order ;
    rdfs:range sales:Customer .
"#,
        )
        .unwrap();

        let def = build_definition_from_rdf(file.to_str().unwrap()).unwrap();
        let parts = def["parts"].as_array().unwrap();
        assert_eq!(parts.len(), 2);

        // First part must be definition.json (Fabric requirement)
        assert_eq!(parts[0]["path"], "definition.json");
        assert_eq!(parts[0]["payloadType"], "InlineBase64");

        // Second part is the RDF file
        assert_eq!(parts[1]["path"], "ontology.ttl");
        assert_eq!(parts[1]["payloadType"], "InlineBase64");

        // Verify base64 decodes back to original content
        let payload = parts[1]["payload"].as_str().unwrap();
        let decoded = BASE64.decode(payload).unwrap();
        let content = String::from_utf8(decoded).unwrap();
        assert!(content.contains("sales:Customer a owl:Class"));
        assert!(content.contains("sales:placedBy a owl:ObjectProperty"));
    }

    #[test]
    fn build_definition_from_rdf_owl() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("ontology.owl");
        std::fs::write(
            &file,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:owl="http://www.w3.org/2002/07/owl#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#">
  <owl:Ontology rdf:about="http://example.org/inventory">
    <rdfs:label>Inventory Ontology</rdfs:label>
  </owl:Ontology>
  <owl:Class rdf:about="http://example.org/inventory#Warehouse">
    <rdfs:label>Warehouse</rdfs:label>
  </owl:Class>
</rdf:RDF>"#,
        )
        .unwrap();

        let def = build_definition_from_rdf(file.to_str().unwrap()).unwrap();
        assert_eq!(def["parts"][1]["path"], "ontology.owl");
    }

    #[test]
    fn build_definition_from_rdf_jsonld() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("graph.jsonld");
        std::fs::write(
            &file,
            r#"{
  "@context": {
    "owl": "http://www.w3.org/2002/07/owl#",
    "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
    "hr": "http://example.org/hr#"
  },
  "@graph": [
    {"@id": "hr:HROntology", "@type": "owl:Ontology", "rdfs:label": "HR Ontology"},
    {"@id": "hr:Employee", "@type": "owl:Class", "rdfs:label": "Employee"},
    {"@id": "hr:Department", "@type": "owl:Class", "rdfs:label": "Department"}
  ]
}"#,
        )
        .unwrap();

        let def = build_definition_from_rdf(file.to_str().unwrap()).unwrap();
        assert_eq!(def["parts"][1]["path"], "ontology.jsonld");
    }

    #[test]
    fn build_definition_from_rdf_rdf_xml() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("data.rdf");
        std::fs::write(
            &file,
            r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#">
  <rdf:Description rdf:about="http://example.org/Resource">
    <rdfs:label>Example Resource</rdfs:label>
  </rdf:Description>
</rdf:RDF>"#,
        )
        .unwrap();

        let def = build_definition_from_rdf(file.to_str().unwrap()).unwrap();
        assert_eq!(def["parts"][1]["path"], "ontology.rdf");
    }

    #[test]
    fn build_definition_from_rdf_xml_ext() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("data.xml");
        std::fs::write(
            &file,
            r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:owl="http://www.w3.org/2002/07/owl#">
  <owl:Ontology rdf:about="http://example.org/test"/>
</rdf:RDF>"#,
        )
        .unwrap();

        let def = build_definition_from_rdf(file.to_str().unwrap()).unwrap();
        assert_eq!(def["parts"][1]["path"], "ontology.rdf");
    }

    #[test]
    fn build_definition_from_rdf_ntriples() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("triples.nt");
        std::fs::write(
            &file,
            r#"<http://example.org/Employee> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .
<http://example.org/Employee> <http://www.w3.org/2000/01/rdf-schema#label> "Employee" .
<http://example.org/name> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#DatatypeProperty> .
"#,
        )
        .unwrap();

        let def = build_definition_from_rdf(file.to_str().unwrap()).unwrap();
        assert_eq!(def["parts"][1]["path"], "ontology.nt");
    }

    #[test]
    fn build_definition_from_rdf_n3() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notation.n3");
        std::fs::write(
            &file,
            r#"@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix : <http://example.org/geo#> .

:GeoOntology a owl:Ontology ;
    rdfs:label "Geography Ontology" .

:Country a owl:Class ;
    rdfs:label "Country" .

:City a owl:Class ;
    rdfs:label "City" .

:locatedIn a owl:ObjectProperty ;
    rdfs:domain :City ;
    rdfs:range :Country .
"#,
        )
        .unwrap();

        let def = build_definition_from_rdf(file.to_str().unwrap()).unwrap();
        assert_eq!(def["parts"][1]["path"], "ontology.n3");
    }

    #[test]
    fn build_definition_from_rdf_trig() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("named.trig");
        std::fs::write(
            &file,
            r#"@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix : <http://example.org/events#> .

GRAPH :EventGraph {
    :Event a owl:Class ;
        rdfs:label "Event" .
    :Venue a owl:Class ;
        rdfs:label "Venue" .
    :hostedAt a owl:ObjectProperty ;
        rdfs:domain :Event ;
        rdfs:range :Venue .
}
"#,
        )
        .unwrap();

        let def = build_definition_from_rdf(file.to_str().unwrap()).unwrap();
        assert_eq!(def["parts"][1]["path"], "ontology.trig");
    }

    #[test]
    fn build_definition_from_rdf_unsupported_extension() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("data.csv");
        std::fs::write(&file, "a,b,c").unwrap();

        let err = build_definition_from_rdf(file.to_str().unwrap()).unwrap_err();
        assert!(err.to_string().contains("Unsupported RDF format"));
        assert!(err.to_string().contains(".csv"));
    }

    #[test]
    fn build_definition_from_rdf_missing_file() {
        let err = build_definition_from_rdf("/nonexistent/path.ttl").unwrap_err();
        assert!(err.to_string().contains("Failed to read RDF file"));
    }

    #[test]
    fn build_definition_from_rdf_binary_content() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("binary.ttl");
        std::fs::write(&file, [0u8, 1, 2, 255, 254, 253]).unwrap();

        let def = build_definition_from_rdf(file.to_str().unwrap()).unwrap();
        let payload = def["parts"][1]["payload"].as_str().unwrap();
        let decoded = BASE64.decode(payload).unwrap();
        assert_eq!(decoded, &[0u8, 1, 2, 255, 254, 253]);
    }
}
