use std::collections::HashMap;

use db::models::{requests::UpdateWorkspace, workspace::Workspace};
use rmcp::{
    ErrorData, handler::server::tool::Parameters, model::CallToolResult, schemars, tool,
    tool_router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::TaskServer;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct McpListWorkspacesRequest {
    #[schemars(description = "Filter by archived state")]
    archived: Option<bool>,
    #[schemars(description = "Filter by pinned state")]
    pinned: Option<bool>,
    #[schemars(description = "Filter by branch name (exact match, case-insensitive)")]
    branch: Option<String>,
    #[schemars(description = "Case-insensitive substring match against workspace name")]
    name_search: Option<String>,
    #[schemars(description = "Maximum number of workspaces to return (default: 50)")]
    limit: Option<i32>,
    #[schemars(description = "Number of results to skip before returning rows (default: 0)")]
    offset: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct StatusSummaryRaw {
    workspace_id: Uuid,
    latest_session_id: Option<Uuid>,
    has_pending_approval: bool,
    files_changed: Option<usize>,
    lines_added: Option<usize>,
    lines_removed: Option<usize>,
    latest_process_completed_at: Option<String>,
    latest_process_status: Option<String>,
    has_running_dev_server: bool,
    has_unseen_turns: bool,
    pr_status: Option<String>,
    pr_number: Option<i64>,
    pr_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StatusSummaryResponseRaw {
    summaries: Vec<StatusSummaryRaw>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct WorkspaceSummary {
    #[schemars(description = "Workspace ID")]
    id: String,
    #[schemars(description = "Workspace branch")]
    branch: String,
    #[schemars(description = "Whether the workspace is archived")]
    archived: bool,
    #[schemars(description = "Whether the workspace is pinned")]
    pinned: bool,
    #[schemars(description = "Optional workspace display name")]
    name: Option<String>,
    #[schemars(description = "Creation timestamp")]
    created_at: String,
    #[schemars(description = "Last update timestamp")]
    updated_at: String,
    #[schemars(description = "Latest session ID (for follow-up commands)")]
    latest_session_id: Option<String>,
    #[schemars(description = "Latest process status: running, completed, failed, or killed")]
    latest_process_status: Option<String>,
    #[schemars(description = "Is a tool approval pending?")]
    has_pending_approval: bool,
    #[schemars(description = "Does this workspace have unseen agent output?")]
    has_unseen_turns: bool,
    #[schemars(description = "Is a dev server currently running?")]
    has_running_dev_server: bool,
    #[schemars(description = "Number of files with changes")]
    files_changed: Option<usize>,
    #[schemars(description = "PR status (e.g. open, merged, closed)")]
    pr_status: Option<String>,
    #[schemars(description = "PR number")]
    pr_number: Option<i64>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpListWorkspacesResponse {
    workspaces: Vec<WorkspaceSummary>,
    total_count: usize,
    returned_count: usize,
    limit: usize,
    offset: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct McpUpdateWorkspaceRequest {
    #[schemars(
        description = "Workspace ID to update. Optional if running inside that workspace context."
    )]
    workspace_id: Option<Uuid>,
    #[schemars(description = "Set archived state")]
    archived: Option<bool>,
    #[schemars(description = "Set pinned state")]
    pinned: Option<bool>,
    #[schemars(description = "Set workspace display name (empty string clears it)")]
    name: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpUpdateWorkspaceResponse {
    success: bool,
    workspace_id: String,
    archived: bool,
    pinned: bool,
    name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct McpDeleteWorkspaceRequest {
    #[schemars(
        description = "Workspace ID to delete. Optional if running inside that workspace context."
    )]
    workspace_id: Option<Uuid>,
    #[schemars(
        description = "Also delete linked remote workspace when available (default: false)"
    )]
    delete_remote: Option<bool>,
    #[schemars(description = "Also delete workspace branches from repos (default: false)")]
    delete_branches: Option<bool>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpDeleteWorkspaceResponse {
    success: bool,
    workspace_id: String,
    delete_remote: bool,
    delete_branches: bool,
}

#[tool_router(router = workspaces_tools_router, vis = "pub")]
impl TaskServer {
    #[tool(description = "List local workspaces with optional filters and pagination. Includes live status: process state, pending approvals, unseen output, file changes, and PR info.")]
    async fn list_workspaces(
        &self,
        Parameters(McpListWorkspacesRequest {
            archived,
            pinned,
            branch,
            name_search,
            limit,
            offset,
        }): Parameters<McpListWorkspacesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let archived_filter = archived.unwrap_or(false);

        let url = self.url("/api/task-attempts");
        let mut workspaces: Vec<Workspace> = match self.send_json(self.client.get(&url)).await {
            Ok(ws) => ws,
            Err(e) => return Ok(e),
        };

        // Fetch status summaries for the same archived state.
        let summary_url = self.url("/api/task-attempts/summary");
        let summary_payload = serde_json::json!({ "archived": archived_filter });
        let status_map: HashMap<Uuid, StatusSummaryRaw> =
            match self.send_json(self.client.post(&summary_url).json(&summary_payload)).await {
                Ok(resp) => {
                    let resp: StatusSummaryResponseRaw = resp;
                    resp.summaries
                        .into_iter()
                        .map(|s| (s.workspace_id, s))
                        .collect()
                }
                Err(_) => HashMap::new(), // Degrade gracefully — show workspaces without status
            };

        // Apply filters.
        workspaces.retain(|w| w.archived == archived_filter);
        if let Some(pinned_filter) = pinned {
            workspaces.retain(|w| w.pinned == pinned_filter);
        }
        if let Some(branch_filter) = branch.as_deref() {
            workspaces.retain(|w| w.branch.eq_ignore_ascii_case(branch_filter));
        }
        if let Some(name_search) = name_search.as_deref() {
            let needle = name_search.to_ascii_lowercase();
            workspaces.retain(|w| {
                w.name
                    .as_deref()
                    .map(|name| name.to_ascii_lowercase().contains(&needle))
                    .unwrap_or(false)
            });
        }

        workspaces.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total_count = workspaces.len();
        let offset = offset.unwrap_or(0).max(0) as usize;
        let limit = limit.unwrap_or(50).max(0) as usize;

        let workspace_summaries = workspaces
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|workspace| {
                let status = status_map.get(&workspace.id);
                WorkspaceSummary {
                    id: workspace.id.to_string(),
                    branch: workspace.branch,
                    archived: workspace.archived,
                    pinned: workspace.pinned,
                    name: workspace.name,
                    created_at: workspace.created_at.to_rfc3339(),
                    updated_at: workspace.updated_at.to_rfc3339(),
                    latest_session_id: status
                        .and_then(|s| s.latest_session_id.map(|id| id.to_string())),
                    latest_process_status: status
                        .and_then(|s| s.latest_process_status.clone()),
                    has_pending_approval: status
                        .map(|s| s.has_pending_approval)
                        .unwrap_or(false),
                    has_unseen_turns: status
                        .map(|s| s.has_unseen_turns)
                        .unwrap_or(false),
                    has_running_dev_server: status
                        .map(|s| s.has_running_dev_server)
                        .unwrap_or(false),
                    files_changed: status.and_then(|s| s.files_changed),
                    pr_status: status.and_then(|s| s.pr_status.clone()),
                    pr_number: status.and_then(|s| s.pr_number),
                }
            })
            .collect::<Vec<_>>();

        TaskServer::success(&McpListWorkspacesResponse {
            returned_count: workspace_summaries.len(),
            total_count,
            limit,
            offset,
            workspaces: workspace_summaries,
        })
    }

    #[tool(
        description = "Update a workspace's archived, pinned, or name fields. `workspace_id` is optional if running inside that workspace context."
    )]
    async fn update_workspace(
        &self,
        Parameters(McpUpdateWorkspaceRequest {
            workspace_id,
            archived,
            pinned,
            name,
        }): Parameters<McpUpdateWorkspaceRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let workspace_id = match workspace_id {
            Some(id) => id,
            None => match self.context.as_ref() {
                Some(ctx) => ctx.workspace_id,
                None => {
                    return Self::err(
                        "workspace_id is required (not available from workspace context)",
                        None::<&str>,
                    );
                }
            },
        };

        let url = self.url(&format!("/api/task-attempts/{}", workspace_id));
        let payload = UpdateWorkspace {
            archived,
            pinned,
            name,
        };

        let updated: Workspace = match self.send_json(self.client.put(&url).json(&payload)).await {
            Ok(ws) => ws,
            Err(e) => return Ok(e),
        };

        TaskServer::success(&McpUpdateWorkspaceResponse {
            success: true,
            workspace_id: updated.id.to_string(),
            archived: updated.archived,
            pinned: updated.pinned,
            name: updated.name,
        })
    }

    #[tool(
        description = "Delete a local workspace. `workspace_id` is optional if running inside that workspace context."
    )]
    async fn delete_workspace(
        &self,
        Parameters(McpDeleteWorkspaceRequest {
            workspace_id,
            delete_remote,
            delete_branches,
        }): Parameters<McpDeleteWorkspaceRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let workspace_id = match workspace_id {
            Some(id) => id,
            None => match self.context.as_ref() {
                Some(ctx) => ctx.workspace_id,
                None => {
                    return Self::err(
                        "workspace_id is required (not available from workspace context)",
                        None::<&str>,
                    );
                }
            },
        };

        let delete_remote = delete_remote.unwrap_or(false);
        let delete_branches = delete_branches.unwrap_or(false);

        let url = self.url(&format!("/api/task-attempts/{}", workspace_id));
        if let Err(e) = self
            .send_empty_json(self.client.delete(&url).query(&[
                ("delete_remote", delete_remote),
                ("delete_branches", delete_branches),
            ]))
            .await
        {
            return Ok(e);
        }

        TaskServer::success(&McpDeleteWorkspaceResponse {
            success: true,
            workspace_id: workspace_id.to_string(),
            delete_remote,
            delete_branches,
        })
    }
}
