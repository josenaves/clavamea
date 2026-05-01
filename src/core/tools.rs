use crate::core::storage::MemoryStorage;
use crate::db::connection::Pool;
use crate::db::models::{Interaction, NewInteraction};
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::InputFile;
use tokio::process::Command;

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
    ScheduleWebSearch,
    FetchUrl,
    SaveRecipe,
    ListRecipes,
    RecordBookEpisode,
    SearchBookEpisodes,
    SaveBookChapter,
    ExportBookManuscript,
    EditCode,
    GitOperate,
    GithubReadIssues,
    GithubUpdateIssue,
    GithubCreatePullRequest,
    DownloadMusic,
    SetUserTimezone,
    CancelSchedule,
    ListSchedules,
    UpdateServer,
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
                    "description": "Executes WebAssembly Text (WAT) code in a sandboxed Wasm runtime via WASI. Only WAT format is supported. This CANNOT run Python, JavaScript, or other languages — only WAT/Wasm.",
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
                            "fuel_type": { "type": "string", "enum": ["gasoline", "alcohol", "diesel", "flex"], "description": "Type of fuel used" }
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
                    "description": "Generates a report of expenses and fuel consumption for the last 12 months.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "vehicle_id": { "type": "integer", "description": "The ID of the vehicle" }
                        },
                        "required": ["vehicle_id"]
                    }
                }
            }),
            Tool::ScheduleReminder => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "schedule_reminder",
                    "description": "Schedules a proactive message/reminder to be sent to the user. Supports both one-time and recurring reminders.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "datetime": {
                                "type": "string",
                                "description": "For ONE-TIME reminders: 'YYYY-MM-DD HH:MM' (e.g., '2026-03-22 10:00'). For RECURRING reminders: 'HH:MM DAYS' (e.g., '09:00 MON-FRI' or '17:10 MON,WED,FRI')."
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
            Tool::ScheduleWebSearch => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "schedule_web_search",
                    "description": "Schedule a recurring reminder that performs a web search when triggered. Use for periodic information updates like sports scores, news, etc.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "message": {
                                "type": "string",
                                "description": "Confirmation message to show when scheduled (e.g., 'Te aviso toda segunda 8:00')"
                            },
                            "time": {
                                "type": "string",
                                "description": "Time in HH:MM format (e.g., '08:00')"
                            },
                            "days": {
                                "type": "string",
                                "description": "Days of week: 'MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT', 'SUN' or 'MON-FRI' for weekdays"
                            },
                            "search_query": {
                                "type": "string",
                                "description": "What to search for (e.g., 'resultados jogos Cruzeiro', 'notícias do bitcoin')"
                            }
                        },
                        "required": ["message", "time", "days", "search_query"]
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
            Tool::RecordBookEpisode => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "record_book_episode",
                    "description": "Records an episodic memory for a book project.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "approximate_date": {
                                "type": "string",
                                "description": "The approximate date or period of the memory (e.g., 'Spring 2021', 'October 2019')."
                            },
                            "content": {
                                "type": "string",
                                "description": "The full text of the memory/episode."
                            },
                            "tags": {
                                "type": "string",
                                "description": "Comma separated list of tags (e.g., 'winter, job, housing')."
                            },
                            "phase": {
                                "type": "string",
                                "description": "The phase of the book this memory belongs to (e.g., 'arrival', 'adaptation', 'return')."
                            }
                        },
                        "required": ["content"]
                    }
                }
            }),
            Tool::SearchBookEpisodes => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "search_book_episodes",
                    "description": "Searches stored episodic memories by tags or phase to use as context when generating chapters.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "tags": {
                                "type": "string",
                                "description": "Comma separated tags to search for."
                            },
                            "phase": {
                                "type": "string",
                                "description": "Phase of the book to search for."
                            }
                        }
                    }
                }
            }),
            Tool::SaveBookChapter => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "save_book_chapter",
                    "description": "Saves a generated book chapter as a Markdown file and registers it in the DB.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "order_num": {
                                "type": "integer",
                                "description": "The order number of the chapter (e.g., 1, 2, 3)."
                            },
                            "title": {
                                "type": "string",
                                "description": "The title of the chapter."
                            },
                            "content": {
                                "type": "string",
                                "description": "The Markdown content of the chapter."
                            }
                        },
                        "required": ["order_num", "title", "content"]
                    }
                }
            }),
            Tool::ExportBookManuscript => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "export_book_manuscript",
                    "description": "Concatenates all book chapters in order into a single manuscript file. Includes summaries and tag information at the end.",
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            }),
            Tool::EditCode => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "edit_code",
                    "description": "Modifies or creates a file in the workspace to write code.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "The path of the file to edit or create."
                            },
                            "content": {
                                "type": "string",
                                "description": "The new content to write."
                            }
                        },
                        "required": ["path", "content"]
                    }
                }
            }),
            Tool::GitOperate => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "git_operate",
                    "description": "Executes a git command in the workspace.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": {
                                "type": "string",
                                "description": "The git command to run (e.g. 'status', 'add .', 'commit -m \"msg\"', 'push')."
                            }
                        },
                        "required": ["command"]
                    }
                }
            }),
            Tool::GithubReadIssues => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "github_read_issues",
                    "description": "Reads open issues assigned to the bot from GitHub.",
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            }),
            Tool::GithubUpdateIssue => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "github_update_issue",
                    "description": "Adds a comment or closes an issue on GitHub. Use this after completing a task or to ask for clarification.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "issue_number": {
                                "type": "integer",
                                "description": "The GitHub issue number."
                            },
                            "comment": {
                                "type": "string",
                                "description": "The comment to write to the issue."
                            },
                            "close": {
                                "type": "boolean",
                                "description": "Whether to close the issue after commenting."
                            }
                        },
                        "required": ["issue_number"]
                    }
                }
            }),
            Tool::GithubCreatePullRequest => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "github_create_pull_request",
                    "description": "Creates a Pull Request on GitHub. IMPORTANT: You MUST create a branch and push your changes using git_operate BEFORE calling this tool.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "title": {
                                "type": "string",
                                "description": "The title of the Pull Request."
                            },
                            "body": {
                                "type": "string",
                                "description": "The description/body of the Pull Request."
                            },
                            "head": {
                                "type": "string",
                                "description": "The name of the branch where your changes are (e.g., 'feature-new-skill')."
                            },
                            "base": {
                                "type": "string",
                                "description": "The branch you want to merge into (usually 'main')."
                            }
                        },
                        "required": ["title", "body", "head", "base"]
                    }
                }
            }),
            Tool::DownloadMusic => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "download_music",
                    "description": "Downloads and converts a YouTube video to a high-quality MP3 (max 10 minutes).",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "url": {
                                "type": "string",
                                "description": "The YouTube URL of the song."
                            }
                        },
                        "required": ["url"]
                    }
                }
            }),
            Tool::SetUserTimezone => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "set_user_timezone",
                    "description": "Sets the user's timezone for accurate scheduling of reminders. Call this when you detect or the user tells you their timezone.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "timezone": {
                                "type": "string",
                                "description": "IANA timezone identifier (e.g., 'America/Sao_Paulo', 'America/New_York', 'Europe/London', 'UTC')"
                            }
                        },
                        "required": ["timezone"]
                    }
                }
            }),
            Tool::CancelSchedule => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "cancel_schedule",
                    "description": "Cancels/deletes a scheduled reminder by its ID. Use this when the user wants to cancel a previously scheduled reminder.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "schedule_id": {
                                "type": "integer",
                                "description": "The ID of the schedule to cancel (shown when the reminder was created)."
                            }
                        },
                        "required": ["schedule_id"]
                    }
                }
            }),
            Tool::ListSchedules => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "list_schedules",
                    "description": "Lists all pending scheduled reminders for the current user. Use this when the user asks what reminders are active.",
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            }),
            Tool::UpdateServer => serde_json::json!({
                "type": "function",
                "function": {
                    "name": "update_server",
                    "description": "Updates or restarts the bot itself by pulling new images and restarting containers. Call this when asked to 'restart', 'reboot', 'update', 'self-update', or 'upgrade'. This is a high-privilege administrative action.",
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
            "schedule_web_search" => Some(Tool::ScheduleWebSearch),
            "fetch_url" => Some(Tool::FetchUrl),
            "save_recipe" => Some(Tool::SaveRecipe),
            "list_recipes" => Some(Tool::ListRecipes),
            "record_book_episode" => Some(Tool::RecordBookEpisode),
            "search_book_episodes" => Some(Tool::SearchBookEpisodes),
            "save_book_chapter" => Some(Tool::SaveBookChapter),
            "export_book_manuscript" => Some(Tool::ExportBookManuscript),
            "edit_code" => Some(Tool::EditCode),
            "git_operate" => Some(Tool::GitOperate),
            "github_read_issues" => Some(Tool::GithubReadIssues),
            "github_update_issue" => Some(Tool::GithubUpdateIssue),
            "github_create_pull_request" => Some(Tool::GithubCreatePullRequest),
            "download_music" => Some(Tool::DownloadMusic),
            "set_user_timezone" => Some(Tool::SetUserTimezone),
            "cancel_schedule" => Some(Tool::CancelSchedule),
            "list_schedules" => Some(Tool::ListSchedules),
            "update_server" => Some(Tool::UpdateServer),
            _ => None,
        }
    }

    /// Execute the tool with the given arguments.
    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        &self,
        bot: &Bot,
        chat_id: ChatId,
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
                self.perform_file_read(user_id, path, allowed_paths).await
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
                let content_res = self.perform_file_read(user_id, path, allowed_paths).await?;
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
                        let code = code.to_string();
                        let wasm = wasm.clone();
                        let result = tokio::task::spawn_blocking(move || wasm.execute_wat(&code))
                            .await
                            .map_err(|e| anyhow!("Wasm execution task panicked: {}", e))??;
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
                self.perform_list_dir(user_id, path, allowed_paths).await
            }
            Tool::MoveFile => {
                let source = args["source"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'source' argument"))?;
                let destination = args["destination"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'destination' argument"))?;
                self.perform_move_file(user_id, source, destination, allowed_paths)
                    .await
            }
            Tool::CreateDir => {
                let path = args["path"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                self.perform_create_dir(user_id, path, allowed_paths).await
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

                let valid_types = ["gasoline", "alcohol", "diesel", "flex"];
                if !valid_types.contains(&fuel_type) {
                    return Err(anyhow!(
                        "Invalid fuel_type '{}'. Must be one of: {}",
                        fuel_type,
                        valid_types.join(", ")
                    ));
                }

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

                let valid_categories = [
                    "maintenance",
                    "tax",
                    "parking",
                    "toll",
                    "insurance",
                    "other",
                ];
                if !valid_categories.contains(&category) {
                    return Err(anyhow!(
                        "Invalid category '{}'. Must be one of: {}",
                        category,
                        valid_categories.join(", ")
                    ));
                }

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

                let message_owned = message.to_string();
                let id = crate::db::queries::insert_schedule(
                    db_pool,
                    user_id,
                    datetime,
                    "reminder",
                    Some(message_owned.as_str()),
                    None,
                )
                .await?;

                Ok(format!("Reminder successfully scheduled (ID: {}).", id))
            }
            Tool::ScheduleWebSearch => {
                let message = args["message"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'message' argument"))?;
                let time = args["time"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'time' argument"))?;
                let days = args["days"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'days' argument"))?;
                let search_query = args["search_query"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'search_query' argument"))?;

                let cron_expr = format!("{} {}", time, days);
                let payload = serde_json::json!({
                    "message": message,
                    "search_query": search_query
                })
                .to_string();

                let id = crate::db::queries::insert_schedule(
                    db_pool,
                    user_id,
                    &cron_expr,
                    "web_search",
                    Some(&payload),
                    Some(search_query.to_string().as_str()),
                )
                .await?;

                Ok(format!("Web search scheduled successfully (ID: {}).", id))
            }
            Tool::FetchUrl => {
                let url = args["url"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'url' argument"))?;
                if !is_safe_url(url) {
                    return Err(anyhow!(
                        "URL not allowed: {}. Only HTTP(S) URLs to public hosts are supported.",
                        url
                    ));
                }
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
            Tool::RecordBookEpisode => {
                let content = args["content"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'content' argument"))?;
                let date = args["approximate_date"].as_str();
                let tags = args["tags"].as_str();
                let phase = args["phase"].as_str();

                let id = crate::db::queries::insert_book_episode(
                    db_pool, user_id, date, content, tags, phase,
                )
                .await?;

                // Ingest into RAG for immediate retrieval
                let _ = rag
                    .ingest_document(
                        user_id,
                        "book_episodes",
                        &format!("episode_{}", id),
                        content,
                    )
                    .await;

                // Get total count
                let count = crate::db::queries::count_book_episodes(db_pool, user_id)
                    .await
                    .unwrap_or(0);

                Ok(format!(
                    "Successfully recorded book episode memory. Database ID: {}. Total episodes: {}",
                    id, count
                ))
            }
            Tool::SearchBookEpisodes => {
                let tags = args["tags"].as_str();
                let phase = args["phase"].as_str();

                let episodes =
                    crate::db::queries::search_book_episodes(db_pool, user_id, tags, phase).await?;

                if episodes.is_empty() {
                    Ok("No matching episodes found.".to_string())
                } else {
                    let text_repr: Vec<String> = episodes
                        .into_iter()
                        .map(|e| {
                            format!(
                                "Episode {} (Date: {}, Phase: {}, Tags: {}):\n{}",
                                e.id,
                                e.approximate_date.unwrap_or_else(|| "Unknown".into()),
                                e.phase.unwrap_or_else(|| "Unknown".into()),
                                e.tags.unwrap_or_else(|| "None".into()),
                                e.content
                            )
                        })
                        .collect();
                    Ok(format!(
                        "Found episodes:\n\n{}",
                        text_repr.join("\n\n---\n\n")
                    ))
                }
            }
            Tool::SaveBookChapter => {
                let order_num = args["order_num"]
                    .as_i64()
                    .ok_or_else(|| anyhow!("Missing 'order_num' argument"))?;
                let title = args["title"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'title' argument"))?;
                let content = args["content"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'content' argument"))?;

                self.perform_save_book_chapter(db_pool, user_id, order_num, title, content, storage)
                    .await
            }
            Tool::ExportBookManuscript => {
                self.perform_export_book_manuscript(db_pool, user_id, storage)
                    .await
            }
            Tool::EditCode => {
                let path = args["path"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'path' argument"))?;
                let content = args["content"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'content' argument"))?;
                self.perform_edit_code(user_id, path, content, allowed_paths)
                    .await
            }
            Tool::GitOperate => {
                let command = args["command"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'command' argument"))?;
                self.perform_git_operate(command).await
            }
            Tool::GithubReadIssues => self.perform_github_read_issues().await,
            Tool::GithubUpdateIssue => {
                let issue_num = args["issue_number"]
                    .as_i64()
                    .ok_or_else(|| anyhow!("Missing 'issue_number' argument"))?;
                let comment = args["comment"].as_str();
                let close = args["close"].as_bool();
                self.perform_github_update_issue(issue_num, comment, close)
                    .await
            }
            Tool::GithubCreatePullRequest => {
                let title = args["title"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'title' argument"))?;
                let body = args["body"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'body' argument"))?;
                let head = args["head"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'head' argument"))?;
                let base = args["base"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'base' argument"))?;
                self.perform_github_create_pull_request(title, body, head, base)
                    .await
            }
            Tool::DownloadMusic => {
                let url = args["url"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'url' argument"))?;
                if !is_youtube_url(url) {
                    return Err(anyhow!(
                        "Only YouTube URLs are supported for music download. Got: {}",
                        url
                    ));
                }
                self.perform_download_music(bot, chat_id, url).await
            }
            Tool::SetUserTimezone => {
                let tz = args["timezone"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing 'timezone' argument"))?;
                crate::db::queries::update_user_timezone(db_pool, user_id, tz).await?;
                Ok(format!(
                    "Timezone set to {} for user {}. All reminders will use this timezone.",
                    tz, user_id
                ))
            }
            Tool::CancelSchedule => {
                let schedule_id = args["schedule_id"]
                    .as_i64()
                    .ok_or_else(|| anyhow!("Missing 'schedule_id' argument"))?;
                crate::db::queries::delete_schedule(db_pool, schedule_id).await?;
                Ok(format!("Schedule {} cancelled successfully.", schedule_id))
            }
            Tool::ListSchedules => {
                let schedules = crate::db::queries::list_user_schedules(db_pool, user_id).await?;
                if schedules.is_empty() {
                    Ok("You have no pending reminders.".to_string())
                } else {
                    let lines: Vec<String> = schedules
                        .iter()
                        .map(|s| {
                            format!(
                                "ID {} — {} {}: {}",
                                s.id,
                                s.task_type,
                                s.cron_expr,
                                s.payload.as_deref().unwrap_or("(no message)")
                            )
                        })
                        .collect();
                    Ok(format!("Your reminders:\n\n{}", lines.join("\n")))
                }
            }
            Tool::UpdateServer => {
                // Check if user is admin
                let user = crate::db::queries::get_user(db_pool, user_id)
                    .await?
                    .ok_or_else(|| anyhow!("User not found"))?;

                if !user.is_admin() {
                    return Err(anyhow!(
                        "Unauthorized: Only admins can trigger server updates."
                    ));
                }

                self.perform_update_server().await
            }
        }
    }

    async fn perform_save_book_chapter(
        &self,
        db_pool: &Pool,
        user_id: i64,
        order_num: i64,
        title: &str,
        content: &str,
        storage: Arc<MemoryStorage>,
    ) -> Result<String> {
        let filename = format!("capitulo_{:02}.md", order_num);
        let sub_path = format!("manuscrito/{}", filename);

        let file_content = format!("# {}\n\n{}", title, content);

        storage.update_file(user_id, &sub_path, &file_content, false)?;

        crate::db::queries::insert_book_chapter(db_pool, user_id, order_num, title, &sub_path)
            .await?;

        Ok(format!(
            "Chapter {} saved successfully as {}.",
            order_num, sub_path
        ))
    }

    async fn perform_export_book_manuscript(
        &self,
        db_pool: &Pool,
        user_id: i64,
        storage: Arc<MemoryStorage>,
    ) -> Result<String> {
        let chapters = crate::db::queries::get_book_chapters(db_pool, user_id).await?;

        if chapters.is_empty() {
            return Ok("No chapters found to export.".to_string());
        }

        let mut final_content = String::new();
        final_content.push_str("# O Segredo da Suécia\n\n");
        final_content.push_str("## Sumário\n");
        for chap in &chapters {
            final_content.push_str(&format!("* Capítulo {} - {}\n", chap.order_num, chap.title));
        }
        final_content.push_str("\n---\n\n");

        for chap in &chapters {
            // Read from storage using perform_file_read's underlying storage logic
            // Directly reading using MemoryStorage path builder since we know it's a relative memory path
            let user_dir = storage.user_dir(user_id);
            let chap_buf = user_dir.join(&chap.filepath);
            if chap_buf.exists() {
                let text = std::fs::read_to_string(&chap_buf)?;
                final_content.push_str(&text);
                final_content.push_str("\n\n\\pagebreak\n\n");
            } else {
                final_content.push_str(&format!(
                    "> [Erro: Arquivo não encontrado para o capítulo {}]\n\n",
                    chap.order_num
                ));
            }
        }

        // Add tags summary from episodes
        final_content.push_str("## Resumo de Tags por Episódio Registrado\n\n");
        let episodes =
            crate::db::queries::search_book_episodes(db_pool, user_id, None, None).await?;
        for e in episodes {
            final_content.push_str(&format!(
                "- Ep {}: Tags: [{}] (Phase: {})\n",
                e.id,
                e.tags.unwrap_or_default(),
                e.phase.unwrap_or_default()
            ));
        }

        storage.update_file(
            user_id,
            "manuscrito/livro_completo.md",
            &final_content,
            false,
        )?;

        Ok("Manuscript successfully exported to manuscrito/livro_completo.md.".to_string())
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
            name, full_path_str
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
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                if let Some(p_str) = path.to_str() {
                    recipes.push(p_str.to_string());
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
        user_id: i64,
        path_str: &str,
        allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
    ) -> Result<String> {
        let canonical_target = self
            .validate_path(user_id, path_str, false, allowed_paths)
            .await?;

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
        user_id: i64,
        path_str: &str,
        allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
    ) -> Result<String> {
        let canonical_target = self
            .validate_path(user_id, path_str, false, allowed_paths)
            .await?;

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
        user_id: i64,
        source_str: &str,
        dest_str: &str,
        allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
    ) -> Result<String> {
        let canonical_source = self
            .validate_path(user_id, source_str, false, allowed_paths.clone())
            .await?;

        // For destination, we allow it to NOT exist yet, so we validate its parent.
        let dest_path = std::path::Path::new(dest_str);
        let dest_parent = dest_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let _ = self
            .validate_path(
                user_id,
                dest_parent.to_str().unwrap_or("."),
                true,
                allowed_paths,
            )
            .await?;

        std::fs::rename(&canonical_source, dest_str)?;
        Ok(format!("Successfully moved {} to {}", source_str, dest_str))
    }

    /// Create a directory.
    async fn perform_create_dir(
        &self,
        user_id: i64,
        path_str: &str,
        allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
    ) -> Result<String> {
        // Validate that we can create here.
        let _ = self
            .validate_path(user_id, path_str, true, allowed_paths)
            .await?;

        std::fs::create_dir_all(path_str)?;
        Ok(format!("Successfully created directory: {}", path_str))
    }

    /// Validates a path against security constraints.
    /// Allows paths within the project or paths starting with AUTHORIZED PATHS (from env or chat).
    /// If DISABLE_PATH_SANDBOX=true is set, all path restrictions are bypassed.
    #[allow(clippy::too_many_arguments)]
    async fn validate_path(
        &self,
        user_id: i64,
        path_str: &str,
        allow_non_existent: bool,
        allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
    ) -> Result<std::path::PathBuf> {
        let sandbox_disabled = std::env::var("DISABLE_PATH_SANDBOX")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(false);
        let base_path = std::env::current_dir()?;

        self.validate_path_internal(
            user_id,
            path_str,
            allow_non_existent,
            allowed_paths,
            &base_path,
            sandbox_disabled,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn validate_path_internal(
        &self,
        user_id: i64,
        path_str: &str,
        allow_non_existent: bool,
        allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
        base_path: &std::path::Path,
        sandbox_disabled: bool,
    ) -> Result<std::path::PathBuf> {
        let path = std::path::Path::new(path_str);

        // Resolve paths
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            base_path.join(path)
        };

        // Smart fallback: If relative path doesn't exist in root, try memory/{user_id}/
        let target_path = if !path.is_absolute() && !absolute_path.exists() {
            let user_base = base_path.join(format!("memory/{}", user_id));
            user_base.join(path)
        } else {
            absolute_path
        };

        // If sandbox is disabled, return the resolved path immediately
        if sandbox_disabled {
            if !allow_non_existent && !target_path.exists() {
                return Err(anyhow!("Path does not exist: {}", path_str));
            }
            return Ok(target_path);
        }

        // Canonicalize if it exists
        let canonical_target = if target_path.exists() {
            target_path.canonicalize()?
        } else {
            if allow_non_existent {
                target_path
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

    async fn perform_edit_code(
        &self,
        user_id: i64,
        path_str: &str,
        content: &str,
        allowed_paths: Arc<tokio::sync::RwLock<Vec<String>>>,
    ) -> Result<String> {
        // Use the shared path validation (respects sandbox settings and allowed_paths)
        let canonical_path = self
            .validate_path(user_id, path_str, true, allowed_paths)
            .await?;

        let parent = canonical_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new(""));
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&canonical_path, content)?;
        Ok(format!("Successfully wrote code to {}.", path_str))
    }

    async fn perform_git_operate(&self, command: &str) -> Result<String> {
        let args_vec = shell_split(command);
        if args_vec.is_empty() {
            return Err(anyhow!("Empty git command."));
        }

        let output = std::process::Command::new("git").args(&args_vec).output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Err(anyhow!(
                "Git command failed.\nStdout: {}\nStderr: {}",
                stdout,
                stderr
            ));
        }

        Ok(format!("Git operation successful.\nOutput:\n{}", stdout))
    }

    async fn perform_github_read_issues(&self) -> Result<String> {
        let token = std::env::var("GITHUB_TOKEN")
            .map_err(|_| anyhow!("GITHUB_TOKEN not configured in environment"))?;

        let client = reqwest::Client::builder()
            .user_agent("ClavaMea/1.6.0")
            .build()?;

        let repo = "josenaves/clavamea"; // Default repo as instructed
        let url = format!("https://api.github.com/repos/{}/issues?state=open", repo);

        let res = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow!("Failed to fetch issues: {}", res.status()));
        }

        let issues: Vec<Value> = res.json().await?;
        if issues.is_empty() {
            return Ok("No open issues found.".to_string());
        }

        let mut output = String::from("Open Issues:\n\n");
        for issue in issues.iter() {
            if issue["pull_request"].is_object() {
                continue; // Skip PRs, only list issues
            }
            let number = issue["number"].as_i64().unwrap_or(0);
            let title = issue["title"].as_str().unwrap_or("No title");
            output.push_str(&format!("#{}: {}\n", number, title));
        }

        Ok(output)
    }

    async fn perform_github_update_issue(
        &self,
        issue_num: i64,
        comment: Option<&str>,
        close: Option<bool>,
    ) -> Result<String> {
        let token = std::env::var("GITHUB_TOKEN")
            .map_err(|_| anyhow!("GITHUB_TOKEN not configured in environment"))?;

        let client = reqwest::Client::builder()
            .user_agent("ClavaMea/1.6.0")
            .build()?;

        let repo = "josenaves/clavamea"; // Default repo

        let mut result_msg = String::new();

        // 1. Post comment if provided
        if let Some(c) = comment {
            let url = format!(
                "https://api.github.com/repos/{}/issues/{}/comments",
                repo, issue_num
            );
            let body = serde_json::json!({ "body": c });
            let res = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Accept", "application/vnd.github.v3+json")
                .json(&body)
                .send()
                .await?;

            if res.status().is_success() {
                result_msg.push_str("Successfully posted comment.\n");
            } else {
                return Err(anyhow!("Failed to post comment: {}", res.status()));
            }
        }

        // 2. Close issue if requested
        if let Some(true) = close {
            let url = format!("https://api.github.com/repos/{}/issues/{}", repo, issue_num);
            let body = serde_json::json!({ "state": "closed" });
            let res = client
                .patch(&url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Accept", "application/vnd.github.v3+json")
                .json(&body)
                .send()
                .await?;

            if res.status().is_success() {
                result_msg.push_str("Successfully closed the issue.\n");
            } else {
                return Err(anyhow!("Failed to close issue: {}", res.status()));
            }
        }

        if result_msg.is_empty() {
            Ok("No action taken (no comment or close command provided).".to_string())
        } else {
            Ok(result_msg)
        }
    }

    async fn perform_github_create_pull_request(
        &self,
        title: &str,
        body: &str,
        head: &str,
        base: &str,
    ) -> Result<String> {
        let token = std::env::var("GITHUB_TOKEN")
            .map_err(|_| anyhow!("GITHUB_TOKEN not configured in environment"))?;

        let client = reqwest::Client::builder()
            .user_agent("ClavaMea/1.6.0")
            .build()?;

        let repo = "josenaves/clavamea";

        let url = format!("https://api.github.com/repos/{}/pulls", repo);
        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "head": head,
            "base": base
        });

        let res = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&payload)
            .send()
            .await?;

        if res.status().is_success() {
            let pr: Value = res.json().await?;
            let pr_url = pr["html_url"].as_str().unwrap_or("unknown URL");
            Ok(format!("Pull Request created successfully: {}", pr_url))
        } else {
            let status = res.status();
            let err_body = res.text().await.unwrap_or_default();
            Err(anyhow!(
                "Failed to create Pull Request: {} - {}",
                status,
                err_body
            ))
        }
    }

    async fn perform_update_server(&self) -> Result<String> {
        let update_path = std::env::var("SERVER_UPDATE_PATH").map_err(|_| {
            anyhow!("SERVER_UPDATE_PATH not configured in environment (check .env)")
        })?;

        tracing::info!("Starting server update at {}", update_path);

        // Resolve docker binary path — the process may run with a restricted PATH
        // inside a container, so we check common locations explicitly.
        let docker_bin = Self::resolve_binary(
            "docker",
            &[
                "/usr/bin/docker",
                "/usr/local/bin/docker",
                "/usr/local/docker/docker",
            ],
        );

        tracing::info!("Using docker binary: {}", docker_bin);

        // Build a sane PATH for child processes
        let child_path =
            std::env::var("PATH").unwrap_or_default() + ":/usr/bin:/usr/local/bin:/bin";

        // 1. docker compose pull
        let pull_output = std::process::Command::new(&docker_bin)
            .args(["compose", "pull"])
            .current_dir(&update_path)
            .env("PATH", &child_path)
            .output()
            .map_err(|e| anyhow!("Failed to run docker ({}): {}", docker_bin, e))?;

        let mut result_msg = String::from("Server update initiated.\n\n");

        if !pull_output.status.success() {
            let stderr = String::from_utf8_lossy(&pull_output.stderr);
            return Err(anyhow!("Failed to pull images: {}", stderr));
        }
        result_msg.push_str("✅ Images pulled successfully.\n");

        // 2. docker compose up -d
        // Note: this will likely terminate this process if the image for this bot changed.
        let up_output = std::process::Command::new(&docker_bin)
            .args(["compose", "up", "-d"])
            .current_dir(&update_path)
            .env("PATH", &child_path)
            .output()
            .map_err(|e| anyhow!("Failed to run docker ({}): {}", docker_bin, e))?;

        if !up_output.status.success() {
            let stderr = String::from_utf8_lossy(&up_output.stderr);
            return Err(anyhow!("Failed to start containers: {}", stderr));
        }
        result_msg.push_str("✅ Containers restarted successfully.\n");
        result_msg.push_str("\nNote: The bot might restart and be offline for a few seconds.");

        Ok(result_msg)
    }

    /// Resolves a binary name by checking well-known absolute paths first,
    /// falling back to the plain name (relies on PATH at runtime).
    fn resolve_binary(name: &str, candidates: &[&str]) -> String {
        for &path in candidates {
            if std::path::Path::new(path).exists() {
                return path.to_string();
            }
        }
        name.to_string()
    }

    async fn perform_download_music(
        &self,
        bot: &Bot,
        chat_id: ChatId,
        url: &str,
    ) -> Result<String> {
        let download_dir = Path::new("data/downloads/music");
        if !download_dir.exists() {
            std::fs::create_dir_all(download_dir)?;
        }

        // Use a unique ID for this download to avoid filename collisions
        let download_uuid = uuid::Uuid::new_v4().to_string();
        let output_template = format!(
            "{}/%(title)s_{}.%(ext)s",
            download_dir.display(),
            download_uuid
        );

        tracing::info!("Downloading music from YouTube: {}", url);

        // Run yt-dlp
        // -x: extract audio
        // --audio-format mp3
        // --audio-quality 0: best quality
        // --match-filter "duration <= 600": 10 minutes limit
        let output = Command::new("yt-dlp")
            .arg("-x")
            .arg("--audio-format")
            .arg("mp3")
            .arg("--audio-quality")
            .arg("0")
            .arg("--match-filter")
            .arg("duration <= 600")
            .arg("-o")
            .arg(&output_template)
            .arg(url)
            .output()
            .await?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            tracing::error!("yt-dlp failed: {}", err);
            if err.contains("Does not pass filter duration") {
                return Ok(
                    "Erro: O vídeo é muito longo. O limite máximo é de 10 minutos.".to_string(),
                );
            }
            return Err(anyhow!("Failed to download music: {}", err));
        }

        // Find the downloaded file
        // Since we used a UUID in the template, we can look for files containing that UUID
        let mut downloaded_file = None;
        for entry in std::fs::read_dir(download_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.contains(&download_uuid) && file_name.ends_with(".mp3") {
                        downloaded_file = Some(path);
                        break;
                    }
                }
            }
        }

        let file_path =
            downloaded_file.ok_or_else(|| anyhow!("Could not find the downloaded MP3 file."))?;

        // Send to Telegram
        tracing::info!("Sending audio file to Telegram: {:?}", file_path);
        bot.send_audio(chat_id, InputFile::file(&file_path)).await?;

        // Clean up
        let _ = std::fs::remove_file(&file_path);

        Ok("Música baixada e enviada com sucesso!".to_string())
    }
}

/// Split a shell command string into arguments, respecting single and double quotes.
fn shell_split(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if in_double {
            if ch == '"' {
                in_double = false;
            } else {
                current.push(ch);
            }
        } else if in_single {
            if ch == '\'' {
                in_single = false;
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            in_double = true;
        } else if ch == '\'' {
            in_single = true;
        } else if ch == '\\' && i + 1 < chars.len() {
            current.push(chars[i + 1]);
            i += 1;
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                args.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
        i += 1;
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

/// Validate that a URL is safe to fetch (no private IPs, no localhost, HTTP(S) only).
fn is_safe_url(url_str: &str) -> bool {
    let url = match url::Url::parse(url_str) {
        Ok(u) => u,
        Err(_) => return false,
    };

    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return false;
    }

    let host = match url.host_str() {
        Some(h) => h.to_lowercase(),
        None => return false,
    };

    if host.is_empty() {
        return false;
    }

    if host == "localhost" || host == "127.0.0.1" || host == "[::1]" {
        return false;
    }

    if host.starts_with("169.254.") {
        return false;
    }

    if host.starts_with("10.") || host.starts_with("172.16.") || host.starts_with("192.168.") {
        if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
            if ip.is_private() {
                return false;
            }
        }
    }

    true
}

/// Check if a URL is a YouTube video URL.
fn is_youtube_url(url_str: &str) -> bool {
    if let Ok(url) = url::Url::parse(url_str) {
        let host = url.host_str().unwrap_or("");
        host == "www.youtube.com"
            || host == "youtube.com"
            || host == "youtu.be"
            || host == "www.youtu.be"
            || host == "m.youtube.com"
    } else {
        false
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
            Tool::ScheduleWebSearch,
            Tool::FetchUrl,
            Tool::SaveRecipe,
            Tool::ListRecipes,
            Tool::RecordBookEpisode,
            Tool::SearchBookEpisodes,
            Tool::SaveBookChapter,
            Tool::ExportBookManuscript,
            Tool::EditCode,
            Tool::GitOperate,
            Tool::GithubReadIssues,
            Tool::GithubUpdateIssue,
            Tool::GithubCreatePullRequest,
            Tool::DownloadMusic,
            Tool::SetUserTimezone,
            Tool::CancelSchedule,
            Tool::ListSchedules,
            Tool::UpdateServer,
        ],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_all_tools_parsable_by_name() {
        for phase in 1..=3 {
            for tool in get_available_tools(phase) {
                let definition = tool.definition();
                let name = definition["function"]["name"]
                    .as_str()
                    .expect("Tool definition must have a string name");

                let parsed = Tool::from_name(name);
                assert!(
                    parsed.is_some(),
                    "Tool '{}' relies on `Tool::from_name` but was not found in the match arm!",
                    name
                );
            }
        }
    }

    // ========== shell_split tests ==========

    #[test]
    fn test_shell_split_simple() {
        let result = shell_split("git status");
        assert_eq!(result, vec!["git", "status"]);
    }

    #[test]
    fn test_shell_split_with_double_quotes() {
        let result = shell_split(r#"commit -m "fix bug in parser""#);
        assert_eq!(result, vec!["commit", "-m", "fix bug in parser"]);
    }

    #[test]
    fn test_shell_split_with_single_quotes() {
        let result = shell_split("echo 'hello world'");
        assert_eq!(result, vec!["echo", "hello world"]);
    }

    #[test]
    fn test_shell_split_mixed_quotes() {
        let result = shell_split(r#"commit -m "it's working""#);
        assert_eq!(result, vec!["commit", "-m", "it's working"]);
    }

    #[test]
    fn test_shell_split_empty() {
        let result = shell_split("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_shell_split_whitespace_only() {
        let result = shell_split("   \t  ");
        assert!(result.is_empty());
    }

    // ========== URL validation tests ==========

    #[test]
    fn test_is_safe_url_https() {
        assert!(is_safe_url("https://example.com/path?q=1"));
    }

    #[test]
    fn test_is_safe_url_http() {
        assert!(is_safe_url("http://example.com"));
    }

    #[test]
    fn test_is_safe_url_rejects_localhost() {
        assert!(!is_safe_url("http://localhost:8080/api"));
    }

    #[test]
    fn test_is_safe_url_rejects_loopback() {
        assert!(!is_safe_url("http://127.0.0.1:3000"));
    }

    #[test]
    fn test_is_safe_url_rejects_private_ip() {
        assert!(!is_safe_url("http://192.168.1.1/admin"));
        assert!(!is_safe_url("http://10.0.0.1:80"));
    }

    #[test]
    fn test_is_safe_url_rejects_metadata() {
        assert!(!is_safe_url("http://169.254.169.254/latest/meta-data"));
    }

    #[test]
    fn test_is_safe_url_rejects_file() {
        assert!(!is_safe_url("file:///etc/passwd"));
    }

    #[test]
    fn test_is_safe_url_rejects_invalid() {
        assert!(!is_safe_url("not-a-url"));
        assert!(!is_safe_url(""));
    }

    // ========== is_youtube_url tests ==========

    #[test]
    fn test_is_youtube_url_valid() {
        assert!(is_youtube_url("https://www.youtube.com/watch?v=abc123"));
        assert!(is_youtube_url("https://youtube.com/watch?v=abc123"));
        assert!(is_youtube_url("https://youtu.be/abc123"));
        assert!(is_youtube_url("https://m.youtube.com/watch?v=abc123"));
    }

    #[test]
    fn test_is_youtube_url_rejects_other() {
        assert!(!is_youtube_url("https://vimeo.com/12345"));
        assert!(!is_youtube_url("https://soundcloud.com/track"));
        assert!(!is_youtube_url("not-a-url"));
        assert!(!is_youtube_url(""));
    }

    // ========== Tool definition tests ==========

    #[test]
    fn test_get_vehicle_report_definition_has_no_period() {
        let def = Tool::GetVehicleReport.definition();
        let params = &def["function"]["parameters"]["properties"];
        // period was removed since it's not implemented
        assert!(params.get("period").is_none());
        assert!(params.get("vehicle_id").is_some());
    }

    #[test]
    fn test_schedule_reminder_requires_datetime_and_message() {
        let def = Tool::ScheduleReminder.definition();
        let required = def["function"]["parameters"]["required"]
            .as_array()
            .unwrap();
        let required: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(required.contains(&"datetime"));
        assert!(required.contains(&"message"));
    }

    #[test]
    fn test_all_tool_definitions_have_valid_schemas() {
        for tool in get_available_tools(3) {
            let def = tool.definition();
            assert!(
                def["type"].as_str() == Some("function"),
                "Tool missing type"
            );
            let func = &def["function"];
            assert!(
                func["name"].as_str().is_some(),
                "Tool missing function name"
            );
            assert!(
                func["parameters"].is_object(),
                "Tool missing parameters object"
            );
        }
    }

    // ========== validate_path tests ==========

    #[tokio::test]
    async fn test_validate_path_user_fallback() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let base_path = temp.path().to_path_buf();

        let user_id = 123456;
        let user_dir = base_path.join(format!("memory/{}", user_id));
        let recipes_dir = user_dir.join("recipes");
        std::fs::create_dir_all(&recipes_dir)?;

        let recipe_file = recipes_dir.join("lasanha.md");
        std::fs::write(&recipe_file, "conteudo da lasanha")?;

        let tool = Tool::FileReader;
        let allowed_paths = Arc::new(tokio::sync::RwLock::new(vec![]));

        let resolved = tool
            .validate_path_internal(
                user_id,
                "recipes/lasanha.md",
                false,
                allowed_paths,
                &base_path,
                false,
            )
            .await?;

        assert!(resolved.exists());
        assert!(
            resolved
                .to_str()
                .unwrap()
                .contains("memory/123456/recipes/lasanha.md")
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_path_rejects_path_outside_sandbox() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let base_path = temp.path().to_path_buf();

        // Create a file outside the project (in /tmp)
        let outside = std::env::temp_dir().join("clavamea_test_outside");
        std::fs::write(&outside, "secret")?;

        let tool = Tool::FileReader;
        let allowed_paths = Arc::new(tokio::sync::RwLock::new(vec![]));

        let result = tool
            .validate_path_internal(
                1,
                outside.to_str().unwrap(),
                false,
                allowed_paths,
                &base_path,
                false,
            )
            .await;

        assert!(result.is_err(), "Should reject path outside sandbox");

        let _ = std::fs::remove_file(&outside);
        Ok(())
    }

    #[tokio::test]
    async fn test_validate_path_with_sandbox_disabled() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let base_path = temp.path().to_path_buf();

        // Create file outside project
        let outside = std::env::temp_dir().join("clavamea_sandbox_off_test");
        std::fs::write(&outside, "accessible")?;

        let tool = Tool::FileReader;
        let allowed_paths = Arc::new(tokio::sync::RwLock::new(vec![]));

        let result = tool
            .validate_path_internal(
                1,
                outside.to_str().unwrap(),
                false,
                allowed_paths,
                &base_path,
                true,
            )
            .await;

        assert!(result.is_ok(), "Should allow path when sandbox is disabled");

        // Cleanup
        let _ = std::fs::remove_file(&outside);
        Ok(())
    }

    // ========== Tool execution tests (with DB) ==========

    async fn make_test_pool() -> sqlx::SqlitePool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "
            CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                username TEXT, role TEXT, authorized INTEGER,
                full_name TEXT, last_seen_version TEXT,
                timezone TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE schedules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                cron_expr TEXT NOT NULL,
                task_type TEXT NOT NULL,
                payload TEXT,
                last_run TEXT,
                search_query TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );
            CREATE TABLE vehicles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                model TEXT,
                plate TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );
            CREATE TABLE fuel_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                vehicle_id INTEGER NOT NULL,
                odometer REAL NOT NULL,
                liters REAL NOT NULL,
                price_per_liter REAL NOT NULL,
                fuel_type TEXT NOT NULL,
                total_cost REAL NOT NULL,
                date DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (vehicle_id) REFERENCES vehicles(id)
            );
            CREATE TABLE expense_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                vehicle_id INTEGER NOT NULL,
                category TEXT NOT NULL,
                description TEXT,
                cost REAL NOT NULL,
                date DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (vehicle_id) REFERENCES vehicles(id)
            );
            CREATE TABLE document_chunks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                filename TEXT NOT NULL,
                path TEXT NOT NULL,
                chunk_index INTEGER NOT NULL DEFAULT 0,
                content TEXT NOT NULL,
                embedding BLOB,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );
        ",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO users (id, role, authorized) VALUES (1, 'owner', 1);")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_schedule_reminder_execution() -> anyhow::Result<()> {
        let pool = make_test_pool().await;
        let tool = Tool::ScheduleReminder;

        let args = serde_json::json!({
            "datetime": "2099-12-31 08:00",
            "message": "Test reminder"
        });

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                Arc::new(MemoryStorage::new(tempfile::tempdir()?.path())?),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await?;

        assert!(result.contains("successfully scheduled"));
        assert!(result.contains("ID:"));

        // Verify it's in the DB
        let _schedules =
            crate::db::queries::get_due_schedules(&pool, "08:00", "MON", "UTC").await?;
        // Since date is 2099-12-31, which is not today, it won't be due now
        // But it should exist
        let all: Vec<crate::db::models::Schedule> = sqlx::query_as("SELECT * FROM schedules")
            .fetch_all(&pool)
            .await?;
        assert!(!all.is_empty());
        assert_eq!(all[0].task_type, "reminder");
        assert_eq!(all[0].payload.as_deref(), Some("Test reminder"));

        Ok(())
    }

    #[tokio::test]
    async fn test_schedule_reminder_recurring() -> anyhow::Result<()> {
        let pool = make_test_pool().await;
        let tool = Tool::ScheduleReminder;

        let args = serde_json::json!({
            "datetime": "09:00 MON-FRI",
            "message": "Daily standup"
        });

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                Arc::new(MemoryStorage::new(tempfile::tempdir()?.path())?),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await?;

        assert!(result.contains("successfully scheduled"));

        // Verify due on a weekday
        let due = crate::db::queries::get_due_schedules(&pool, "09:00", "MON", "UTC").await?;
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].payload.as_deref(), Some("Daily standup"));

        Ok(())
    }

    #[tokio::test]
    async fn test_get_vehicle_report_basic() -> anyhow::Result<()> {
        let pool = make_test_pool().await;
        let tool = Tool::GetVehicleReport;

        // Add a vehicle first
        crate::db::queries::insert_vehicle(&pool, 1, "Meu Carro", Some("Modelo X"), None).await?;

        let args = serde_json::json!({"vehicle_id": 1});

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                Arc::new(MemoryStorage::new(tempfile::tempdir()?.path())?),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await?;

        assert!(
            result.contains("Relatório do Veículo"),
            "Expected report to contain 'Relatório do Veículo', got: {}",
            result
        );
        assert!(
            result.contains("Total geral"),
            "Expected report to contain 'Total geral', got: {}",
            result
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_log_fuel_and_get_report() -> anyhow::Result<()> {
        let pool = make_test_pool().await;
        crate::db::queries::insert_vehicle(&pool, 1, "Meu Carro", None, None).await?;

        // Log fuel
        let tool = Tool::LogFuel;
        let args = serde_json::json!({
            "vehicle_id": 1,
            "odometer": 1000.0,
            "liters": 50.0,
            "price_per_liter": 5.50,
            "fuel_type": "gasoline"
        });

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                Arc::new(MemoryStorage::new(tempfile::tempdir()?.path())?),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await?;

        assert!(result.contains("R$ 275.00")); // 50 * 5.50

        // Now check the report
        let report_tool = Tool::GetVehicleReport;
        let report_args = serde_json::json!({"vehicle_id": 1});

        let report = report_tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &report_args,
                Arc::new(MemoryStorage::new(tempfile::tempdir()?.path())?),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await?;

        assert!(report.contains("R$ 275.00"));

        Ok(())
    }

    #[tokio::test]
    async fn test_genetics_calculate_hardy_weinberg() -> anyhow::Result<()> {
        let pool = make_test_pool().await;
        let tool = Tool::GeneticsCalculate;

        let args = serde_json::json!({
            "calculation_type": "hardy_weinberg",
            "affected": 1600.0,
            "population": 10000.0
        });

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                Arc::new(MemoryStorage::new(tempfile::tempdir()?.path())?),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await?;

        // q = sqrt(1600/10000) = sqrt(0.16) = 0.4
        // p = 1 - 0.4 = 0.6
        assert!(result.contains("0.4") || result.contains("40%")); // q
        assert!(result.contains("0.6") || result.contains("60%")); // p

        Ok(())
    }

    #[tokio::test]
    async fn test_genetics_calculate_punnett() -> anyhow::Result<()> {
        let pool = make_test_pool().await;
        let tool = Tool::GeneticsCalculate;

        let args = serde_json::json!({
            "calculation_type": "punnett",
            "parent1": "Aa",
            "parent2": "Aa"
        });

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                Arc::new(MemoryStorage::new(tempfile::tempdir()?.path())?),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await?;

        assert!(result.contains("AA"));
        assert!(result.contains("Aa"));
        assert!(result.contains("aa"));

        Ok(())
    }

    #[tokio::test]
    async fn test_log_fuel_missing_args_returns_helpful_error() {
        let pool = make_test_pool().await;
        let tool = Tool::LogFuel;

        let args = serde_json::json!({}); // empty — missing all fields

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                Arc::new(MemoryStorage::new(tempfile::tempdir().unwrap().path()).unwrap()),
                Arc::new(crate::core::RagManager::new(pool.clone()).unwrap()),
                Arc::new(crate::core::wasm::WasmRuntime::new().unwrap()),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("vehicle_id"));
    }

    #[tokio::test]
    async fn test_edit_code_rejects_absolute_path() -> anyhow::Result<()> {
        // Ensure sandbox is enabled for this test (undo any side effects from parallel tests)
        unsafe { std::env::remove_var("DISABLE_PATH_SANDBOX") };

        let pool = make_test_pool().await;
        let tool = Tool::EditCode;
        let allowed_paths = Arc::new(tokio::sync::RwLock::new(vec![]));

        let args = serde_json::json!({
            "path": "/etc/passwd",
            "content": "malicious"
        });

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                Arc::new(MemoryStorage::new(tempfile::tempdir()?.path())?),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                allowed_paths,
                &pool,
            )
            .await;

        assert!(result.is_err(), "Expected error for absolute path");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("negado")
                || err.contains("autorizado")
                || err.contains("Acesso")
                || err.contains("Permission")
                || err.contains("permission"),
            "Expected access denied, got: {}",
            err
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_edit_code_allows_relative_path_in_project() -> anyhow::Result<()> {
        let pool = make_test_pool().await;
        let tool = Tool::EditCode;
        let allowed_paths = Arc::new(tokio::sync::RwLock::new(vec![]));

        // Sandbox must be off to avoid macOS symlink canonicalization issues
        // with non-existent files on /Users -> /private/Users paths
        unsafe { std::env::set_var("DISABLE_PATH_SANDBOX", "true") };

        let args = serde_json::json!({
            "path": "test_output_tools.rs",
            "content": "// test file"
        });

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                Arc::new(MemoryStorage::new(tempfile::tempdir()?.path())?),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                allowed_paths,
                &pool,
            )
            .await?;

        unsafe { std::env::remove_var("DISABLE_PATH_SANDBOX") };

        assert!(
            result.contains("Successfully wrote code"),
            "Expected success, got: {}",
            result
        );

        // The smart fallback in validate_path puts non-existent relative paths into memory/<user_id>/
        let written = std::path::Path::new("memory/1/test_output_tools.rs");
        let written_root = std::path::Path::new("test_output_tools.rs");
        assert!(
            written.exists() || written_root.exists(),
            "File should exist at {} or {}",
            written.display(),
            written_root.display()
        );

        let _ = std::fs::remove_file("memory/1/test_output_tools.rs");
        let _ = std::fs::remove_file("test_output_tools.rs");
        Ok(())
    }

    #[tokio::test]
    async fn test_save_memory_valid_and_invalid_targets() -> anyhow::Result<()> {
        let pool = make_test_pool().await;
        let temp = tempfile::tempdir()?;
        let storage = Arc::new(MemoryStorage::new(temp.path())?);

        let tool = Tool::SaveMemory;

        // Valid target: MEMORY.md
        let args = serde_json::json!({
            "target": "MEMORY.md",
            "content": "Test memory entry"
        });

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                storage.clone(),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await?;

        assert!(result.contains("Successfully appended"));

        // Invalid target
        let bad_args = serde_json::json!({
            "target": "INVALID.md",
            "content": "Bad target"
        });

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &bad_args,
                storage,
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid target"));

        Ok(())
    }

    #[tokio::test]
    async fn test_log_fuel_rejects_invalid_fuel_type() -> anyhow::Result<()> {
        let pool = make_test_pool().await;
        crate::db::queries::insert_vehicle(&pool, 1, "Carro", None, None).await?;

        let tool = Tool::LogFuel;
        let args = serde_json::json!({
            "vehicle_id": 1,
            "odometer": 100.0,
            "liters": 10.0,
            "price_per_liter": 5.0,
            "fuel_type": "invalid_type"
        });

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                Arc::new(MemoryStorage::new(tempfile::tempdir()?.path())?),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid fuel_type")
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_cancel_schedule() -> anyhow::Result<()> {
        let pool = make_test_pool().await;
        crate::db::queries::insert_schedule(
            &pool,
            1,
            "2099-01-01 08:00",
            "reminder",
            Some("test"),
            None,
        )
        .await?;

        let tool = Tool::CancelSchedule;
        let args = serde_json::json!({"schedule_id": 1});

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                Arc::new(MemoryStorage::new(tempfile::tempdir()?.path())?),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await?;

        assert!(result.contains("cancelled"));

        // Verify it's gone
        let all: Vec<crate::db::models::Schedule> = sqlx::query_as("SELECT * FROM schedules")
            .fetch_all(&pool)
            .await?;
        assert!(all.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_list_schedules_empty() -> anyhow::Result<()> {
        let pool = make_test_pool().await;

        let tool = Tool::ListSchedules;
        let args = serde_json::json!({});

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                Arc::new(MemoryStorage::new(tempfile::tempdir()?.path())?),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await?;

        assert!(result.contains("no pending reminders"));

        Ok(())
    }

    #[tokio::test]
    async fn test_list_schedules_with_reminders() -> anyhow::Result<()> {
        let pool = make_test_pool().await;
        crate::db::queries::insert_schedule(
            &pool,
            1,
            "08:00 MON-FRI",
            "reminder",
            Some("daily reminder"),
            None,
        )
        .await?;
        crate::db::queries::insert_schedule(
            &pool,
            1,
            "2099-12-25 10:00",
            "reminder",
            Some("christmas"),
            None,
        )
        .await?;

        let tool = Tool::ListSchedules;
        let args = serde_json::json!({});

        let result = tool
            .execute(
                &teloxide::Bot::new("dummy"),
                teloxide::types::ChatId(1),
                1,
                &args,
                Arc::new(MemoryStorage::new(tempfile::tempdir()?.path())?),
                Arc::new(crate::core::RagManager::new(pool.clone())?),
                Arc::new(crate::core::wasm::WasmRuntime::new()?),
                Arc::new(tokio::sync::RwLock::new(vec![])),
                &pool,
            )
            .await?;

        assert!(result.contains("daily reminder"));
        assert!(result.contains("christmas"));
        assert!(result.contains("MON-FRI"));

        Ok(())
    }
}
