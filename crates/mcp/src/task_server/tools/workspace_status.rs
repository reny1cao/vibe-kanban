use rmcp::{
    ErrorData, handler::server::tool::Parameters, model::CallToolResult, schemars, tool,
    tool_router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::TaskServer;

// --- get_workspace_status types ---

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetWorkspaceStatusRequest {
    #[schemars(description = "Filter by archived state (default: false = active workspaces)")]
    archived: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceSummaryRaw {
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
struct WorkspaceSummaryResponseRaw {
    summaries: Vec<WorkspaceSummaryRaw>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpWorkspaceStatus {
    #[schemars(description = "Workspace ID")]
    workspace_id: String,
    #[schemars(description = "Latest session ID (for follow-up)")]
    latest_session_id: Option<String>,
    #[schemars(description = "Is a tool approval pending?")]
    has_pending_approval: bool,
    #[schemars(description = "Number of files with changes")]
    files_changed: Option<usize>,
    #[schemars(description = "Total lines added")]
    lines_added: Option<usize>,
    #[schemars(description = "Total lines removed")]
    lines_removed: Option<usize>,
    #[schemars(description = "When the latest process completed (ISO 8601)")]
    latest_process_completed_at: Option<String>,
    #[schemars(description = "Status: Running, Completed, Failed, or Killed")]
    latest_process_status: Option<String>,
    #[schemars(description = "Is a dev server currently running?")]
    has_running_dev_server: bool,
    #[schemars(description = "Does this workspace have unseen agent output?")]
    has_unseen_turns: bool,
    #[schemars(description = "PR status (e.g. Open, Merged, Closed)")]
    pr_status: Option<String>,
    #[schemars(description = "PR number")]
    pr_number: Option<i64>,
    #[schemars(description = "PR URL")]
    pr_url: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct GetWorkspaceStatusResponse {
    statuses: Vec<McpWorkspaceStatus>,
    count: usize,
}

// --- get_agent_output types ---

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetAgentOutputRequest {
    #[schemars(
        description = "Workspace ID to get agent output for. Optional if running inside that workspace context."
    )]
    workspace_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct AgentTurnRaw {
    prompt: Option<String>,
    summary: Option<String>,
    seen: bool,
    created_at: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpAgentOutput {
    #[schemars(description = "The prompt sent to the agent")]
    prompt: Option<String>,
    #[schemars(description = "The agent's output/summary")]
    summary: Option<String>,
    #[schemars(description = "Whether this output has been seen")]
    seen: bool,
    #[schemars(description = "When this turn was created")]
    created_at: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct GetAgentOutputResponse {
    workspace_id: String,
    turns: Vec<McpAgentOutput>,
    count: usize,
}

// --- mark_turns_seen types ---

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct MarkTurnsSeenRequest {
    #[schemars(
        description = "Workspace ID to mark turns as seen. Optional if running inside that workspace context."
    )]
    workspace_id: Option<Uuid>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct MarkTurnsSeenResponse {
    success: bool,
    workspace_id: String,
}

// --- get_workspace_changes types ---

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetWorkspaceChangesRequest {
    #[schemars(
        description = "Workspace ID to get change details for. Optional if running inside that workspace context."
    )]
    workspace_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct RepoBranchStatusRaw {
    repo_id: Uuid,
    repo_name: String,
    commits_behind: Option<usize>,
    commits_ahead: Option<usize>,
    has_uncommitted_changes: Option<bool>,
    head_oid: Option<String>,
    uncommitted_count: Option<usize>,
    untracked_count: Option<usize>,
    target_branch_name: String,
    is_rebase_in_progress: bool,
    conflicted_files: Vec<String>,
    is_target_remote: bool,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpRepoChanges {
    #[schemars(description = "Repository ID")]
    repo_id: String,
    #[schemars(description = "Repository name")]
    repo_name: String,
    #[schemars(description = "Commits behind target branch")]
    commits_behind: Option<usize>,
    #[schemars(description = "Commits ahead of target branch")]
    commits_ahead: Option<usize>,
    #[schemars(description = "Whether there are uncommitted changes")]
    has_uncommitted_changes: Option<bool>,
    #[schemars(description = "Number of uncommitted file changes")]
    uncommitted_count: Option<usize>,
    #[schemars(description = "Number of untracked files")]
    untracked_count: Option<usize>,
    #[schemars(description = "Current HEAD commit SHA")]
    head_oid: Option<String>,
    #[schemars(description = "Target branch name for merging")]
    target_branch_name: String,
    #[schemars(description = "Is a git rebase currently in progress?")]
    is_rebase_in_progress: bool,
    #[schemars(description = "Files currently in conflict")]
    conflicted_files: Vec<String>,
    #[schemars(description = "Is the target branch remote-only (must use PR)?")]
    is_target_remote: bool,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct GetWorkspaceChangesResponse {
    workspace_id: String,
    repos: Vec<McpRepoChanges>,
    count: usize,
}

// --- stop_execution types ---

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct StopExecutionRequest {
    #[schemars(description = "Execution process ID to stop")]
    execution_process_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct StopExecutionResponse {
    success: bool,
    execution_process_id: String,
}

// --- send_follow_up types ---

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SendFollowUpRequest {
    #[schemars(description = "Session ID to send follow-up to. Get from get_workspace_status (latest_session_id).")]
    session_id: Uuid,
    #[schemars(description = "The follow-up prompt/instruction to send to the agent")]
    prompt: String,
    #[schemars(description = "The executor type (e.g. 'CLAUDE_CODE', 'CODEX', 'OPENCODE'). Must match the session's executor.")]
    executor: String,
    #[schemars(description = "Optional executor variant")]
    variant: Option<String>,
}

#[derive(Debug, Serialize)]
struct FollowUpPayload {
    prompt: String,
    executor_config: FollowUpExecutorConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_process_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct FollowUpExecutorConfig {
    executor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    variant: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExecutionProcessRaw {
    id: Uuid,
    status: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct SendFollowUpResponse {
    success: bool,
    execution_process_id: String,
    status: String,
}

#[tool_router(router = workspace_status_tools_router, vis = "pub")]
impl TaskServer {
    /// REN-1: Get workspace status information including process state, diff stats, and PR info.
    #[tool(
        description = "Get status information for all workspaces: process status (Running/Completed/Failed/Killed), pending approvals, file changes, PR info, and unseen agent output. Use this to monitor workspace health."
    )]
    async fn get_workspace_status(
        &self,
        Parameters(GetWorkspaceStatusRequest { archived }): Parameters<GetWorkspaceStatusRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let archived = archived.unwrap_or(false);
        let url = self.url("/api/task-attempts/summary");
        let payload = serde_json::json!({ "archived": archived });

        let response: WorkspaceSummaryResponseRaw =
            match self.send_json(self.client.post(&url).json(&payload)).await {
                Ok(r) => r,
                Err(e) => return Ok(e),
            };

        let statuses: Vec<McpWorkspaceStatus> = response
            .summaries
            .into_iter()
            .map(|s| McpWorkspaceStatus {
                workspace_id: s.workspace_id.to_string(),
                latest_session_id: s.latest_session_id.map(|id| id.to_string()),
                has_pending_approval: s.has_pending_approval,
                files_changed: s.files_changed,
                lines_added: s.lines_added,
                lines_removed: s.lines_removed,
                latest_process_completed_at: s.latest_process_completed_at,
                latest_process_status: s.latest_process_status,
                has_running_dev_server: s.has_running_dev_server,
                has_unseen_turns: s.has_unseen_turns,
                pr_status: s.pr_status,
                pr_number: s.pr_number,
                pr_url: s.pr_url,
            })
            .collect();

        let count = statuses.len();
        TaskServer::success(&GetWorkspaceStatusResponse { statuses, count })
    }

    /// REN-2: Get agent output (coding agent turns) for a workspace.
    #[tool(
        description = "Get the agent's output/summary for a workspace. Returns the prompts sent and the agent's responses. `workspace_id` is optional if running inside that workspace context."
    )]
    async fn get_agent_output(
        &self,
        Parameters(GetAgentOutputRequest { workspace_id }): Parameters<GetAgentOutputRequest>,
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

        let url = self.url(&format!(
            "/api/task-attempts/{}/agent-output",
            workspace_id
        ));
        let turns: Vec<AgentTurnRaw> = match self.send_json(self.client.get(&url)).await {
            Ok(t) => t,
            Err(e) => return Ok(e),
        };

        let mcp_turns: Vec<McpAgentOutput> = turns
            .into_iter()
            .map(|t| McpAgentOutput {
                prompt: t.prompt,
                summary: t.summary,
                seen: t.seen,
                created_at: t.created_at,
            })
            .collect();

        let count = mcp_turns.len();
        TaskServer::success(&GetAgentOutputResponse {
            workspace_id: workspace_id.to_string(),
            turns: mcp_turns,
            count,
        })
    }

    /// REN-7: Mark all agent output turns for a workspace as seen.
    #[tool(
        description = "Mark all agent output as seen for a workspace. Clears the 'unseen' indicator. `workspace_id` is optional if running inside that workspace context."
    )]
    async fn mark_turns_seen(
        &self,
        Parameters(MarkTurnsSeenRequest { workspace_id }): Parameters<MarkTurnsSeenRequest>,
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

        let url = self.url(&format!("/api/task-attempts/{}/mark-seen", workspace_id));
        if let Err(e) = self.send_empty_json(self.client.put(&url)).await {
            return Ok(e);
        }

        TaskServer::success(&MarkTurnsSeenResponse {
            success: true,
            workspace_id: workspace_id.to_string(),
        })
    }

    /// REN-5: Get detailed change information for a workspace (branch status, commits, conflicts).
    #[tool(
        description = "Get detailed change info for a workspace: commits ahead/behind, uncommitted changes, untracked files, rebase status, and conflicted files per repo. `workspace_id` is optional if running inside that workspace context."
    )]
    async fn get_workspace_changes(
        &self,
        Parameters(GetWorkspaceChangesRequest { workspace_id }): Parameters<
            GetWorkspaceChangesRequest,
        >,
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

        let url = self.url(&format!(
            "/api/task-attempts/{}/branch-status",
            workspace_id
        ));
        let statuses: Vec<RepoBranchStatusRaw> =
            match self.send_json(self.client.get(&url)).await {
                Ok(s) => s,
                Err(e) => return Ok(e),
            };

        let repos: Vec<McpRepoChanges> = statuses
            .into_iter()
            .map(|s| McpRepoChanges {
                repo_id: s.repo_id.to_string(),
                repo_name: s.repo_name,
                commits_behind: s.commits_behind,
                commits_ahead: s.commits_ahead,
                has_uncommitted_changes: s.has_uncommitted_changes,
                uncommitted_count: s.uncommitted_count,
                untracked_count: s.untracked_count,
                head_oid: s.head_oid,
                target_branch_name: s.target_branch_name,
                is_rebase_in_progress: s.is_rebase_in_progress,
                conflicted_files: s.conflicted_files,
                is_target_remote: s.is_target_remote,
            })
            .collect();

        let count = repos.len();
        TaskServer::success(&GetWorkspaceChangesResponse {
            workspace_id: workspace_id.to_string(),
            repos,
            count,
        })
    }

    /// REN-6: Stop a running execution process.
    #[tool(
        description = "Stop a running execution process (kills the agent). Get execution_process_id from workspace status or session info."
    )]
    async fn stop_execution(
        &self,
        Parameters(StopExecutionRequest {
            execution_process_id,
        }): Parameters<StopExecutionRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!(
            "/api/execution-processes/{}/stop",
            execution_process_id
        ));
        if let Err(e) = self.send_empty_json(self.client.post(&url)).await {
            return Ok(e);
        }

        TaskServer::success(&StopExecutionResponse {
            success: true,
            execution_process_id: execution_process_id.to_string(),
        })
    }

    /// REN-4: Send a follow-up prompt to an existing workspace session.
    #[tool(
        description = "Send a follow-up prompt to a running or completed workspace session. Requires session_id (from get_workspace_status) and the executor type. Creates a new execution process."
    )]
    async fn send_follow_up(
        &self,
        Parameters(SendFollowUpRequest {
            session_id,
            prompt,
            executor,
            variant,
        }): Parameters<SendFollowUpRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/sessions/{}/follow-up", session_id));
        let payload = FollowUpPayload {
            prompt,
            executor_config: FollowUpExecutorConfig {
                executor: executor.trim().replace('-', "_").to_ascii_uppercase(),
                variant,
            },
            retry_process_id: None,
        };

        let ep: ExecutionProcessRaw =
            match self.send_json(self.client.post(&url).json(&payload)).await {
                Ok(r) => r,
                Err(e) => return Ok(e),
            };

        TaskServer::success(&SendFollowUpResponse {
            success: true,
            execution_process_id: ep.id.to_string(),
            status: ep.status,
        })
    }
}
