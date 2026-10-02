use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::{json, Value};

#[derive(Subcommand)]
pub(crate) enum SheetCommand {
    List,
    Create {
        title: String,
        #[arg(long)]
        id: Option<String>,
        #[arg(long, default_value = "company")]
        visibility: String,
    },
    #[command(
        long_about = "Read the current workbook, access and head revision. Optional --query JSON (or @file):\n  {\"action\":\"get_range\",\"range\":\"A1:G20\"}\n  {\"action\":\"export_csv\"}\n  {\"action\":\"filter\",\"range\":\"A1:G20\",\"column\":1,\"equals\":\"Sydney\"}\nAdd worksheet: ID to select a worksheet; otherwise the first worksheet is used. Cells include raw content, evaluated value, formatted text and formula errors."
    )]
    Read {
        sheet: String,
        /// Optional read action JSON: get_range, filter, export_csv, snapshot.
        #[arg(long)]
        query: Option<String>,
    },
    /// Apply one guarded, idempotent action. Use @path to read JSON from a file.
    /// Actions: set_cells, insert_rows, sort, create_worksheet, import_csv, update_record.
    #[command(
        long_about = "Apply an atomic upstream workbook action. Supply the head revision from sheet read and a fresh UUID key. Retain identical revision, key and action after a lost response; stale positional edits conflict. Pass JSON or @path:\n  {\"action\":\"set_cells\",\"start\":\"A1\",\"values\":[[\"Item\",\"Buy\",\"Sell\",\"Profit\"],[\"GFX\",2100,3200,\"=C2-B2\"]]}\n  {\"action\":\"insert_rows\",\"before\":2,\"count\":1}\n  {\"action\":\"sort\",\"range\":\"A2:G20\",\"by\":\"F2\",\"direction\":\"desc\"}\n  {\"action\":\"create_worksheet\",\"name\":\"Sales\"}\n  {\"action\":\"import_csv\",\"start\":\"A1\",\"csv\":\"Item,Buy\\nGFX,2100\"}\n  {\"action\":\"update_record\",\"range\":\"A1:G20\",\"record_id\":\"gfx\",\"values\":{\"Sell\":3100}}\nAdd worksheet: ID to select a worksheet. set_cells values accept text/numbers/booleans or {content, format}. update_record expects unique header names and an immutable record_id column (override with id_column)."
    )]
    Edit {
        sheet: String,
        #[arg(long)]
        revision: String,
        #[arg(long)]
        key: String,
        action: String,
    },
    Share {
        sheet: String,
        actor: String,
        #[arg(long, default_value = "edit")]
        access: String,
    },
    Unshare {
        sheet: String,
        actor: String,
    },
    Versions {
        sheet: String,
    },
    Version {
        sheet: String,
        version: String,
    },
    Restore {
        sheet: String,
        version: String,
        title: String,
        #[arg(long)]
        id: Option<String>,
    },
    Checkpoint {
        sheet: String,
        title: String,
        #[arg(long)]
        revision: String,
        #[arg(long)]
        key: String,
    },
}
fn parse(s: &str) -> Result<Value> {
    let bytes = if let Some(path) = s.strip_prefix('@') {
        std::fs::read(path).context("read sheet action")?
    } else {
        s.as_bytes().to_vec()
    };
    Ok(serde_json::from_slice(&bytes).context("parse sheet action JSON")?)
}
impl SheetCommand {
    pub(crate) fn request(self, company: String) -> Result<Value> {
        let operation = match self {
            Self::List => json!({"operation":"list"}),
            Self::Create {
                title,
                id,
                visibility,
            } => {
                json!({"operation":"create","id":id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),"title":title,"visibility":visibility})
            }
            Self::Read { sheet, query } => {
                json!({"operation":"read","sheet":sheet,"query":query.map(|s|parse(&s)).transpose()?})
            }
            Self::Edit {
                sheet,
                revision,
                key,
                action,
            } => {
                json!({"operation":"edit","sheet":sheet,"expected_revision":revision,"key":key,"action":parse(&action)?})
            }
            Self::Share {
                sheet,
                actor,
                access,
            } => json!({"operation":"share","sheet":sheet,"actor":actor,"access":access}),
            Self::Unshare { sheet, actor } => {
                json!({"operation":"share","sheet":sheet,"actor":actor,"access":null})
            }
            Self::Versions { sheet } => json!({"operation":"versions","sheet":sheet}),
            Self::Version { sheet, version } => {
                json!({"operation":"version","sheet":sheet,"version":version})
            }
            Self::Restore {
                sheet,
                version,
                title,
                id,
            } => {
                json!({"operation":"restore_copy","sheet":sheet,"version":version,"title":title,"id":id.unwrap_or_else(||uuid::Uuid::new_v4().to_string())})
            }
            Self::Checkpoint {
                sheet,
                title,
                revision,
                key,
            } => {
                json!({"operation":"checkpoint","sheet":sheet,"title":title,"expected_revision":revision,"key":key})
            }
        };
        Ok(json!({"cmd":"sheet-operation","company":company,"sheet_operation":operation}))
    }
}
