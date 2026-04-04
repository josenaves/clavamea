use crate::core::storage::MemoryStorage;
use crate::db::connection::Pool;
use crate::db::models::{Interaction, NewInteraction};
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

use crate::core::rag::RagManager;

/// Available tools for function calling.
#[derive(Debug, Clone)]
pub enum Tool {
    WebSearch,
    FileReader,
    SaveMemory,
    IndexDocument,
    SearchKnowledge,
    ExecuteCode,
    ListDir,
    MoveFile,
    CreateDir,
    AuthorizePath,
    AddVehicle,
    LogFuel,
    LogExpense,
    GetVehicleReport,
    GeneticsCalculate,
    ScheduleReminder,
    FetchUrl,
    SaveRecipe,
    ListRecipes,
    // Future tools will be added here
}

impl Tool {
    /// Get the tool definition in OpenAI function calling format.
    pub fn definition(&self) -> Value {
        match self {
            Tool::WebSearch => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "web_search",
                    "description": "Search the web for current information",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The search query"
                            }
                        },
                        "required": ["query"]
                    }
                }
            }),
            Tool::FileReader => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "file_reader",
                    "description": "Read files from the local filesystem",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the file to read"
                            }
                        },
                        "required": ["path"]
                    }
                }
            }),
            Tool::SaveMemory => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "save_memory",
                    "description": "Save information to long-term memory or daily notes",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "target": {
                                "type": "string",
                                "enum": ["MEMORY.md", "USER.md", "SOUL.md", "DAILY"],
                                "description": "The file to save to"
                            },
                            "content": {
                                "type": "string",
                                "description": "The content to save"
                            }
                        },
                        "required": ["target", "content"]
                    }
                }
            }),
            Tool::IndexDocument => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "index_document",
                    "description": "Indexes a document for future semantic search (RAG)",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "The path to the file to index"
                            }
                        },
                        "required": ["path"]
                    }
                }
            }),
            Tool::SearchKnowledge => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "search_knowledge",
                    "description": "Searches through indexed documents for relevant information",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The search query"
                            }
                        },
                        "required": ["query"]
                    }
                }
            }),
            Tool::ExecuteCode => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "execute_code",
                    "description": "Executes code in a secure sandboxed environment (Wasm) using WASI. \n\n\
                                  IMPORTANT for 'wat' language:\n\
                                  1. You MUST export a function named '_start' (or 'main').\n\
                                  2. Use (export \"_start\" (func $func_name)). DO NOT use the (start) section.\n\
                                  3. To print to stdout, you must import and use 'wasi_snapshot_preview1' -> 'fd_write'.\n\
                                  4. CRITICAL: In WAT string literals, you MUST double-escape newlines as \\\\n to prevent JSON from evaluating them into literal newlines.\n\n\
                                  Example WAT for 'Hello World':\n\
                                  (module\n\
                                    (import \"wasi_snapshot_preview1\" \"fd_write\" (func $fd_write (param i32 i32 i32 i32) (result i32)))\n\
                                    (memory 1)\n\
                                    (export \"memory\" (memory 0))\n\
                                    (data (i32.const 8) \"Hello, Wasm!\\\\n\")\n\
                                    (func $main (export \"_start\")\n\
                                      (i32.const 0) (i32.const 8) (i32.store)\n\
                                      (i32.const 4) (i32.const 13) (i32.store)\n\
                                      (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 24)\n\
                                      (call $fd_write)\n\
                                      drop\n\
                                    )\n\
                                  )",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "language": {
                                "type": "string",
                                "enum": ["wat"],
                                "description": "The language of the code (currently only 'wat' is supported)"
                            },
                            "code": {
                                "type": "string",
                                "description": "The WebAssembly Text (WAT) code to execute."
                            }
                        },
                        "required": ["language", "code"]
                    }
                }
            }),
            Tool::ListDir => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "list_dir",
                    "description": "Lists contents of a directory (files and subdirectories)",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the directory to list"
                            }
                        },
                        "required": ["path"]
                    }
                }
            }),
            Tool::MoveFile => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "move_file",
                    "description": "Moves or renames a file or directory",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "source": {
                                "type": "string",
                                "description": "Source path"
                            },
                            "destination": {
                                "type": "string",
                                "description": "Destination path"
                            }
                        },
                        "required": ["source", "destination"]
                    }
                }
            }),
            Tool::CreateDir => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "create_dir",
                    "description": "Creates a new directory (and any necessary parent directories)",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the directory to create"
                            }
                        },
                        "required": ["path"]
                    }
                }
            }),
            Tool::AuthorizePath => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "authorize_path",
                    "description": "Authorizes a path for file operations at runtime. Only call this after the user has explicitly given permission in the chat.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "The absolute path to authorize"
                            }
                        },
                        "required": ["path"]
                    }
                }
            }),
            Tool::AddVehicle => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "add_vehicle",
                    "description": "Registers a new vehicle for expense tracking.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "A nickname for the car (e.g., 'Meu Jetta')" },
                            "model": { "type": "string", "description": "The car model (e.g., 'VW Jetta')" },
                            "plate": { "type": "string", "description": "The license plate (optional)" }
                        },
                        "required": ["name"]
                    }
                }
            }),
            Tool::LogFuel => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "log_fuel",
                    "description": "Records a refueling event.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "vehicle_id": { "type": "integer", "description": "The ID of the vehicle" },
                            "odometer": { "type": "number", "description": "The current odometer reading in km" },
                            "liters": { "type": "number", "description": "Amount of fuel in liters" },
                            "price_per_liter": { "type": "number", "description": "Price per liter" },
                            "fuel_type": { "type": "string", "enum": ["gasoline", "alcohol"], "description": "Type of fuel used" }
                        },
                        "required": ["vehicle_id", "odometer", "liters", "price_per_liter", "fuel_type"]
                    }
                }
            }),
            Tool::LogExpense => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "log_expense",
                    "description": "Records a non-fuel expense (parking, toll, maintenance, etc.).",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "vehicle_id": { "type": "integer", "description": "The ID of the vehicle" },
                            "category": { "type": "string", "enum": ["maintenance", "tax", "parking", "toll", "insurance", "other"] },
                            "cost": { "type": "number", "description": "Total cost of the expense" },
                            "description": { "type": "string", "description": "Minor details about the expense" }
                        },
                        "required": ["vehicle_id", "category", "cost"]
                    }
                }
            }),
            Tool::GetVehicleReport => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "get_vehicle_report",
                    "description": "Generates a report of expenses and fuel consumption.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "vehicle_id": { "type": "integer", "description": "The ID of the vehicle" },
                            "period": { "type": "string", "enum": ["all", "month", "year"], "description": "Report period" }
                        },
                        "required": ["vehicle_id"]
                    }
                }
            }),
            Tool::ScheduleReminder => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "schedule_reminder",
                    "description": "Schedules a proactive message/reminder to be sent to the user at a specific future date and time.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "datetime": {
                                "type": "string",
                                "description": "The exact date and time to send the message in 'YYYY-MM-DD HH:MM' format (e.g., '2026-03-22 10:00'), OR a recurring time 'HH:MM MON-FRI'."
                            },
                            "message": {
                                "type": "string",
                                "description": "The text message to send to the user."
                            }
                        },
                        "required": ["datetime", "message"]
                    }
                }
            }),
            Tool::GeneticsCalculate => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "genetics_calculate",
                    "description": "Performs genetics calculations. Supports: (1) Hardy-Weinberg equilibrium given affected individuals and population size; (2) Punnett square for a cross between two genotypes.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "calculation_type": {
                                "type": "string",
                                "enum": ["hardy_weinberg", "punnett"],
                                "description": "Type of calculation to perform."
                            },
                            "affected": {
                                "type": "number",
                                "description": "[hardy_weinberg] Number of affected individuals (homozygous recessive)."
                            },
                            "population": {
                                "type": "number",
                                "description": "[hardy_weinberg] Total population size."
                            },
                            "parent1": {
                                "type": "string",
                                "description": "[punnett] Genotype of first parent, e.g. 'Aa', 'AA', 'aa'."
                            },
                            "parent2": {
                                "type": "string",
                                "description": "[punnett] Genotype of second parent, e.g. 'Aa', 'AA', 'aa'."
                            }
                        },
                        "required": ["calculation_type"]
                    }
                }
            }),
            Tool::FetchUrl => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "fetch_url",
                    "description": "Fetches the raw text content of a web page for processing.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "The URL of the web page to fetch."
                            }
                        },
                        "required": ["url"]
                    }
                }
            }),
            Tool::SaveRecipe => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "save_recipe",
                    "description": "Saves a cleaned and sanitized recipe to the user's recipe collection and indexes it.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "A descriptive name for the recipe (e.g., 'lasanha_de_berinjela')."
                            },
                            "content": {
                                "type": "string",
                                "description": "The cleaned recipe content in Markdown format."
                            }
                        },
                        "required": ["name", "content"]
                    }
                }
            }),
            Tool::ListRecipes => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "list_recipes",
                    "description": "Lists all recipes currently saved in the user's collection.",
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            }),
        }
    }

    /// Parse a tool by its JSON definition name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "web_search" => Some(Tool::WebSearch),
            "file_reader" => Some(Tool::FileReader),
            "save_memory" => Some(Tool::SaveMemory),
            "index_document" => Some(Tool::IndexDocument),
            "search_knowledge" => Some(Tool::SearchKnowledge),
            "execute_code" => Some(Tool::ExecuteCode),
            "list_dir" => Some(Tool::ListDir),
            "move_file" => Some(Tool::MoveFile),
            "create_dir" => Some(Tool::CreateDir),
            "authorize_path" => Some(Tool::AuthorizePath),
            "add_vehicle" => Some(Tool::AddVehicle),
            "log_fuel" => Some(Tool::LogFuel),
            "log_expense" => Some(Tool::LogExpense),
            "get_vehicle_report" => Some(Tool::GetVehicleReport),
            "genetics_calculate" => Some(Tool::GeneticsCalculate),
            "schedule_reminder" => Some(Tool::ScheduleReminder),
            "fetch_url" => Some(Tool::FetchUrl),
            "save_recipe" => Some(Tool::SaveRecipe),
            "list_recipes" => Some(Tool::ListRecipes),
            _ => None,
        }
    }

    /// Execute the tool with the given arguments.
    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        &self,
        user_id: i64,
        args: &Value,
        storage: Arc<MemoryStorage>,
        rag: Arc<RagManager>,
        wasm: Arc<crate::core::wasm::WasmRuntime>,
        allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
        db_pool: &Pool,
    ) -> Result<String> {
        match self {
            Tool::WebSearch => {
                let query = args["query"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'query' argument"))?;
                self.perform_web_search(query).await
            }
            Tool::FileReader => {
                let path = args["path"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                self.perform_file_read(path, allowed_paths).await
            }
            Tool::SaveMemory => {
                let target = args["target"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'target' argument"))?;
                let content = args["content"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'content' argument"))?;

                match target {
                    "DAILY" => {
                        storage.append_daily_note(user_id, content)?;
                        Ok("Successfully saved to daily note.".to_string())
                    }
                    "MEMORY.md" | "USER.md" | "SOUL.md" => {
                        storage.update_file(user_id, target, content, true)?;
                        Ok(format!("Successfully appended to {}.", target))
                    }
                    _ => Err(anyhow!("Invalid target: {}", target)),
                }
            }
            Tool::IndexDocument => {
                let path = args["path"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                let content_res = self.perform_file_read(path, allowed_paths).await?;
                // perform_file_read returns a formatted string with markdown code blocks, extract the content
                let content = content_res
                    .split("```\n")
                    .nth(1)
                    .and_then(|s| s.split("\n```").next())
                    .unwrap_or("");

                let filename = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path);

                rag.ingest_document(user_id, filename, path, content)
                    .await?;
                Ok(format!("Successfully indexed document: {}", path))
            }
            Tool::SearchKnowledge => {
                let query = args["query"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'query' argument"))?;
                let results = rag.search(user_id, query, 3).await?;
                if results.is_empty() {
                    Ok("No relevant knowledge found in indexed documents.".to_string())
                } else {
                    Ok(format!(
                        "Found the following relevant information:\n\n{}",
                        results.join("\n\n---\n\n")
                    ))
                }
            }
            Tool::ExecuteCode => {
                let language = args["language"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'language' argument"))?;
                let code = args["code"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'code' argument"))?;

                match language {
                    "wat" => {
                        let result = wasm.execute_wat(code)?;
                        if result.is_empty() {
                            Ok("Code executed successfully, but produced no output.".to_string())
                        } else {
                            Ok(format!("Execution Result (stdout):\n```\n{}\n```", result))
                        }
                    }
                    _ => Err(anyhow!("Unsupported language: {}", language)),
                }
            }
            Tool::ListDir => {
                let path = args["path"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                self.perform_list_dir(path, allowed_paths).await
            }
            Tool::MoveFile => {
                let source = args["source"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'source' argument"))?;
                let destination = args["destination"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'destination' argument"))?;
                self.perform_move_file(source, destination, allowed_paths)
                    .await
            }
            Tool::CreateDir => {
                let path = args["path"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                self.perform_create_dir(path, allowed_paths).await
            }
            Tool::AuthorizePath => {
                let path = args["path"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                let mut paths = allowed_paths.write().await;
                paths.push(path.to_string());
                Ok(format!("Successfully authorized path: {}", path))
            }
            Tool::AddVehicle => {
                let name = args["name"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'name' argument"))?;
                let model = args["model"].as_str();
                let plate = args["plate"].as_str();

                let id = crate::db::queries::insert_vehicle(db_pool, user_id, name, model, plate)
                    .await?;
                Ok(format!(
                    "Vehicle '{}' added successfully with ID: {}",
                    name, id
                ))
            }
            Tool::LogFuel => {
                let vehicle_id = args["vehicle_id"]
                    .as_i64()
                    .ok_or_else(|| anyhow!("Missing 'vehicle_id' argument"))?;

                // Verify ownership
                if !crate::db::queries::is_vehicle_owner(db_pool, vehicle_id, user_id).await? {
                    return Err(anyhow!("Access denied: You do not own this vehicle."));
                }

                let odometer = args["odometer"]
                    .as_f64()
                    .ok_or_else(|| anyhow!("Missing 'odometer' argument"))?;
                let liters = args["liters"]
                    .as_f64()
                    .ok_or_else(|| anyhow!("Missing 'liters' argument"))?;
                let price_per_liter = args["price_per_liter"]
                    .as_f64()
                    .ok_or_else(|| anyhow!("Missing 'price_per_liter' argument"))?;
                let fuel_type = args["fuel_type"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'fuel_type' argument"))?;

                let total_cost = liters * price_per_liter;

                // Calculate km/L if there's a previous log
                let last_log = crate::db::queries::get_last_fuel_log(db_pool, vehicle_id).await?;
                let consumption_msg = if let Some(last) = last_log {
                    let km_diff = odometer - last.odometer;
                    if km_diff > 0.0 {
                        format!(
                            "\nConsumo desde o último abastecimento: {:.2} km/L",
                            km_diff / liters
                        )
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                };

                crate::db::queries::insert_fuel_log(
                    db_pool,
                    vehicle_id,
                    odometer,
                    liters,
                    price_per_liter,
                    fuel_type,
                    total_cost,
                )
                .await?;
                Ok(format!(
                    "Fuel log saved. Total cost: R$ {:.2}{}",
                    total_cost, consumption_msg
                ))
            }
            Tool::LogExpense => {
                let vehicle_id = args["vehicle_id"]
                    .as_i64()
                    .ok_or_else(|| anyhow!("Missing 'vehicle_id' argument"))?;

                // Verify ownership
                if !crate::db::queries::is_vehicle_owner(db_pool, vehicle_id, user_id).await? {
                    return Err(anyhow!("Access denied: You do not own this vehicle."));
                }

                let category = args["category"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'category' argument"))?;
                let cost = args["cost"]
                    .as_f64()
                    .ok_or_else(|| anyhow!("Missing 'cost' argument"))?;
                let description = args["description"].as_str();

                crate::db::queries::insert_expense_log(
                    db_pool,
                    vehicle_id,
                    category,
                    description,
                    cost,
                )
                .await?;
                Ok(format!("Expense log saved for category '{}'.", category))
            }
            Tool::GetVehicleReport => {
                let vehicle_id = args["vehicle_id"]
                    .as_i64()
                    .ok_or_else(|| anyhow!("Missing 'vehicle_id' argument"))?;

                // Verify ownership
                if !crate::db::queries::is_vehicle_owner(db_pool, vehicle_id, user_id).await? {
                    return Err(anyhow!("Access denied: You do not own this vehicle."));
                }

                // Simplified report: total expenses
                let now = chrono::Utc::now();
                let start = now - chrono::Duration::days(365); // Last year for now

                let fuel_logs =
                    crate::db::queries::get_vehicle_fuel_logs(db_pool, vehicle_id, start, now)
                        .await?;
                let expenses =
                    crate::db::queries::get_vehicle_expenses(db_pool, vehicle_id, start, now)
                        .await?;

                let total_fuel: f64 = fuel_logs.iter().map(|l| l.total_cost).sum();
                let total_liters: f64 = fuel_logs.iter().map(|l| l.liters).sum();
                let total_other: f64 = expenses.iter().map(|e| e.cost).sum();

                let mut report = format!("🚗 **Relatório do Veículo (ID: {})**\n", vehicle_id);
                report.push_str(&format!(
                    "⛽ Gastos com combustível: R$ {:.2} ({:.2} L)\n",
                    total_fuel, total_liters
                ));
                report.push_str(&format!("🛠️ Outros gastos: R$ {:.2}\n", total_other));
                report.push_str(&format!(
                    "💰 **Total geral: R$ {:.2}**\n",
                    total_fuel + total_other
                ));

                if fuel_logs.len() >= 2 {
                    let first_odo = fuel_logs.first().unwrap().odometer;
                    let last_odo = fuel_logs.last().unwrap().odometer;
                    let total_km = last_odo - first_odo;
                    if total_km > 0.0 {
                        report.push_str(&format!(
                            "📈 Média de consumo geral: {:.2} km/L",
                            total_km / total_liters
                        ));
                    }
                }

                Ok(report)
            }
            Tool::GeneticsCalculate => {
                let calc_type = args["calculation_type"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'calculation_type' argument"))?;

                match calc_type {
                    "hardy_weinberg" => {
                        let affected = args["affected"].as_f64().ok_or_else(|| {
                            anyhow!("Missing 'affected' argument for hardy_weinberg")
                        })?;
                        let population = args["population"].as_f64().ok_or_else(|| {
                            anyhow!("Missing 'population' argument for hardy_weinberg")
                        })?;

                        match crate::core::genetics::hardy_weinberg(affected, population) {
                            Ok(result) => Ok(crate::core::genetics::format_hardy_weinberg(
                                &result, affected, population,
                            )),
                            Err(e) => Err(anyhow!("{}", e)),
                        }
                    }
                    "punnett" => {
                        let parent1 = args["parent1"]
                            .as_str()
                            .ok_or_else(|| anyhow!("Missing 'parent1' argument for punnett"))?;
                        let parent2 = args["parent2"]
                            .as_str()
                            .ok_or_else(|| anyhow!("Missing 'parent2' argument for punnett"))?;

                        match crate::core::genetics::punnett(parent1, parent2) {
                            Ok(result) => Ok(crate::core::genetics::format_punnett(
                                &result, parent1, parent2,
                            )),
                            Err(e) => Err(anyhow!("{}", e)),
                        }
                    }
                    _ => Err(anyhow!(
                        "Unknown calculation_type: '{}'. Use 'hardy_weinberg' or 'punnett'.",
                        calc_type
                    )),
                }
            }
            Tool::ScheduleReminder => {
                let datetime = args["datetime"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'datetime' argument"))?;
                let message = args["message"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'message' argument"))?;

                let id = crate::db::queries::insert_schedule(
                    db_pool,
                    user_id,
                    datetime,
                    "reminder",
                    Some(message),
                )
                .await?;

                Ok(format!("Reminder successfully scheduled (ID: {}).", id))
            }
            Tool::FetchUrl => {
                let url = args["url"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'url' argument"))?;
                self.perform_fetch_url(url).await
            }
            Tool::SaveRecipe => {
                let name = args["name"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'name' argument"))?;
                let content = args["content"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'content' argument"))?;

                self.perform_save_recipe(user_id, name, content, storage, rag)
                    .await
            }
            Tool::ListRecipes => self.perform_list_recipes(user_id, storage).await,
        }
    }

    /// Fetches the raw text content of a URL for processing.
    async fn perform_fetch_url(&self, url: &str) -> Result<String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("ClavaMea/1.5.1 (Private AI Assistant)")
            .build()?;

        let res = client.get(url).send().await?;

        if !res.status().is_success() {
            return Err(anyhow!("Failed to fetch URL {}: {}", url, res.status()));
        }

        let body_bytes = res.bytes().await?;

        // Use html2text to get a cleaner version for the LLM
        let text = html2text::from_read(&body_bytes[..], 80);

        // Limit content to 50KB to prevent context overflow.
        if text.len() > 50_000 {
            Ok(format!("{}... [TRUNCATED]", &text[..50_000]))
        } else {
            Ok(text)
        }
    }

    /// Saves a sanitized recipe to the user's collection and indexes it.
    async fn perform_save_recipe(
        &self,
        user_id: i64,
        name: &str,
        content: &str,
        storage: Arc<MemoryStorage>,
        rag: Arc<RagManager>,
    ) -> Result<String> {
        let filename = format!("{}.md", name.to_lowercase().replace(' ', "_"));
        let sub_path = format!("recipes/{}", filename);

        // Save file
        storage.update_file(user_id, &sub_path, content, false)?;

        // Index for RAG
        let user_dir = storage.user_dir(user_id);
        let full_path = user_dir.join(&sub_path);
        let full_path_str = full_path.to_string_lossy();

        rag.ingest_document(user_id, &filename, &full_path_str, content)
            .await?;

        Ok(format!(
            "Recipe '{}' saved to {} and successfully indexed.",
            name, sub_path
        ))
    }

    /// Lists all recipes currently saved for the user.
    async fn perform_list_recipes(
        &self,
        user_id: i64,
        storage: Arc<MemoryStorage>,
    ) -> Result<String> {
        let recipes_dir = storage.user_dir(user_id).join("recipes");

        if !recipes_dir.exists() {
            return Ok("No recipes folder found for this user.".to_string());
        }

        let mut recipes = Vec::new();
        for entry in std::fs::read_dir(recipes_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    recipes.push(name.to_string());
                }
            }
        }

        if recipes.is_empty() {
            Ok("You have no recipes saved yet.".to_string())
        } else {
            Ok(format!(
                "You have the following recipes saved:\n- {}",
                recipes.join("\n- ")
            ))
        }
    }

    /// Read a local file.
    async fn perform_file_read(
        &self,
        path_str: &str,
        allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
    ) -> Result<String> {
        let canonical_target = self.validate_path(path_str, false, allowed_paths).await?;

        use std::io::Read;
        let file = std::fs::File::open(&canonical_target)?;
        let mut buffer = Vec::new();
        // Limit reading to 10KB to avoid flooding the LLM context.
        file.take(10 * 1024).read_to_end(&mut buffer)?;

        let content = String::from_utf8_lossy(&buffer).to_string();
        Ok(format!("Content of {}:\n\n```\n{}\n```", path_str, content))
    }

    /// List contents of a directory.
    async fn perform_list_dir(
        &self,
        path_str: &str,
        allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
    ) -> Result<String> {
        let canonical_target = self.validate_path(path_str, false, allowed_paths).await?;

        if !canonical_target.is_dir() {
            return Err(anyhow!("Path is not a directory: {}", path_str));
        }

        let mut entries = Vec::new();
        for entry in std::fs::read_dir(canonical_target)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let file_type = if metadata.is_dir() { "DIR" } else { "FILE" };
            let size = metadata.len();
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(format!("[{}] {} ({} bytes)", file_type, name, size));
        }

        if entries.is_empty() {
            Ok(format!("Directory {} is empty.", path_str))
        } else {
            Ok(format!(
                "Contents of {}:\n\n{}",
                path_str,
                entries.join("\n")
            ))
        }
    }

    /// Move or rename a file/directory.
    async fn perform_move_file(
        &self,
        source_str: &str,
        dest_str: &str,
        allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
    ) -> Result<String> {
        let canonical_source = self
            .validate_path(source_str, false, allowed_paths.clone())
            .await?;

        // For destination, we allow it to NOT exist yet, so we validate its parent.
        let dest_path = std::path::Path::new(dest_str);
        let dest_parent = dest_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let _ = self
            .validate_path(dest_parent.to_str().unwrap_or("."), true, allowed_paths)
            .await?;

        std::fs::rename(&canonical_source, dest_str)?;
        Ok(format!("Successfully moved {} to {}", source_str, dest_str))
    }

    /// Create a directory.
    async fn perform_create_dir(
        &self,
        path_str: &str,
        allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
    ) -> Result<String> {
        // Validate that we can create here.
        let _ = self.validate_path(path_str, true, allowed_paths).await?;

        std::fs::create_dir_all(path_str)?;
        Ok(format!("Successfully created directory: {}", path_str))
    }

    /// Validates a path against security constraints.
    /// Allows paths within the project or paths starting with AUTHORIZED PATHS (from env or chat).
    #[allow(clippy::too_many_arguments)]
    async fn validate_path(
        &self,
        path_str: &str,
        allow_non_existent: bool,
        allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
    ) -> Result<std::path::PathBuf> {
        let path = std::path::Path::new(path_str);
        let base_path = std::env::current_dir()?;

        // Resolve absolute path
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            base_path.join(path)
        };

        // Canonicalize if it exists
        let canonical_target = if absolute_path.exists() {
            absolute_path.canonicalize()?
        } else {
            if allow_non_existent {
                absolute_path
            } else {
                return Err(anyhow!("Path does not exist: {}", path_str));
            }
        };

        // Rule 1: Allow if within project root
        let canonical_base = base_path.canonicalize()?;
        if canonical_target.starts_with(&canonical_base) {
            return Ok(canonical_target);
        }

        // Rule 2: Allow if within dynamic allowed paths
        let paths = allowed_paths.read().await;
        for allowed_root in paths.iter() {
            let allowed_path = std::path::Path::new(allowed_root);
            if let Ok(canonical_allowed) = allowed_path.canonicalize() {
                if canonical_target.starts_with(&canonical_allowed) {
                    return Ok(canonical_target);
                }
            } else if canonical_target.starts_with(allowed_path) {
                // Fallback for paths that might not exist yet but were authorized
                return Ok(canonical_target);
            }
        }

        Err(anyhow!(
            "Acesso negado: O caminho {} não está autorizado. Por favor, peça autorização explicitamente pelo chat.",
            path_str
        ))
    }

    /// Perform a web search using the Brave Search API.
    async fn perform_web_search(&self, query: &str) -> Result<String> {
        let api_key = std::env::var("BRAVE_API_KEY")
            .map_err(|_| anyhow!("BRAVE_API_KEY not found in environment"))?;

        let client = reqwest::Client::new();
        let res = client
            .get("https://api.search.brave.com/res/v1/web/search")
            .header("Accept", "application/json")
            .header("X-Subscription-Token", api_key)
            .query(&[("q", query), ("count", "5")])
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow!("Brave Search API error {}: {}", status, text));
        }

        let data: Value = res.json().await?;
        let mut results = Vec::new();

        if let Some(web_results) = data["web"]["results"].as_array() {
            for (i, result) in web_results.iter().enumerate() {
                let title = result["title"].as_str().unwrap_or("No Title");
                let description = result["description"].as_str().unwrap_or("No Description");
                let url = result["url"].as_str().unwrap_or("#");
                results.push(format!(
                    "{}. {} ({})\n   Snippet: {}",
                    i + 1,
                    title,
                    url,
                    description
                ));
            }
        }

        if results.is_empty() {
            Ok("No search results found.".to_string())
        } else {
            Ok(format!(
                "Search results for '{}':\n\n{}",
                query,
                results.join("\n\n")
            ))
        }
    }
}

/// Get all available tools for the current phase.
pub fn get_available_tools(phase: u8) -> Vec<Tool> {
    match phase {
        1 => vec![], // MVP: No tools
        2 => vec![Tool::WebSearch, Tool::FileReader, Tool::SaveMemory],
        3 => vec![
            Tool::WebSearch,
            Tool::FileReader,
            Tool::SaveMemory,
            Tool::IndexDocument,
            Tool::SearchKnowledge,
            Tool::ExecuteCode,
            Tool::ListDir,
            Tool::MoveFile,
            Tool::CreateDir,
            Tool::AuthorizePath,
            Tool::AddVehicle,
            Tool::LogFuel,
            Tool::LogExpense,
            Tool::GetVehicleReport,
            Tool::GeneticsCalculate,
            Tool::ScheduleReminder,
            Tool::FetchUrl,
            Tool::SaveRecipe,
            Tool::ListRecipes,
        ],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_tools_parsable_by_name() {
        // Enumerate all active tool phases
        for phase in 1..=3 {
            for tool in get_available_tools(phase) {
                // Ensure every tool defined actually has its parsed name mapped in `from_name`.
                // This prevents bugs where the LLM is given the tool definition but the handler
                // doesn't recognize the callback name.
                let definition = tool.definition();
                let name = definition["function"]["name"]
                    .as_str()
                    .expect("Tool definition must have a string name");

                let parsed = Tool::from_name(name);
                assert!(
                    parsed.is_some(),
                    "Tool '{}' relies on `Tool::from_name` but was not found in the match arm! \
                     Add it to `from_name` in src/core/tools.rs to fix this.",
                    name
                );
            }
        }
    }
}
