use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct PermissionRequest {
    pub is_open: bool,
    pub tool_name: String,
    pub patterns: Vec<String>,
    pub tool_use_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlanModeRequest {
    pub is_open: bool,
    pub plan_content: String,
}

#[derive(Clone)]
pub struct UsePermissionsReturn {
    pub allowed_tools: Signal<Vec<String>>,
    pub permission_request: Signal<Option<PermissionRequest>>,
    pub plan_mode_request: Signal<Option<PlanModeRequest>>,
    pub is_permission_mode: Signal<bool>,
}

impl UsePermissionsReturn {
    pub fn show_permission_request(&self, tool_name: String, patterns: Vec<String>, tool_use_id: String) {
        let mut permission_request = self.permission_request;
        let mut is_permission_mode = self.is_permission_mode;

        permission_request.set(Some(PermissionRequest {
            is_open: true,
            tool_name,
            patterns,
            tool_use_id,
        }));
        is_permission_mode.set(true);
    }

    pub fn close_permission_request(&self) {
        let mut permission_request = self.permission_request;
        let mut is_permission_mode = self.is_permission_mode;

        permission_request.set(None);
        is_permission_mode.set(false);
    }

    pub fn show_plan_mode_request(&self, plan_content: String) {
        let mut plan_mode_request = self.plan_mode_request;
        let mut is_permission_mode = self.is_permission_mode;

        plan_mode_request.set(Some(PlanModeRequest {
            is_open: true,
            plan_content,
        }));
        is_permission_mode.set(true);
    }

    pub fn close_plan_mode_request(&self) {
        let mut plan_mode_request = self.plan_mode_request;
        let mut is_permission_mode = self.is_permission_mode;

        plan_mode_request.set(None);
        is_permission_mode.set(false);
    }

    pub fn allow_tool_temporary(&self, pattern: String, base_tools: Option<Vec<String>>) -> Vec<String> {
        let current_allowed_tools = base_tools.unwrap_or_else(|| self.allowed_tools.read().clone());
        let mut updated_tools = current_allowed_tools.clone();
        updated_tools.push(pattern);
        updated_tools
    }

    pub fn allow_tool_permanent(&self, pattern: String, base_tools: Option<Vec<String>>) -> Vec<String> {
        let mut allowed_tools = self.allowed_tools;
        let current_allowed_tools = base_tools.unwrap_or_else(|| allowed_tools.read().clone());
        let mut updated_tools = current_allowed_tools.clone();
        updated_tools.push(pattern.clone());
        allowed_tools.set(updated_tools.clone());
        updated_tools
    }

    pub fn reset_permissions(&self) {
        let mut allowed_tools = self.allowed_tools;
        allowed_tools.set(vec![]);
    }
}

pub fn use_permissions() -> UsePermissionsReturn {
    let allowed_tools = use_signal(Vec::<String>::new);
    let permission_request = use_signal(|| None::<PermissionRequest>);
    let plan_mode_request = use_signal(|| None::<PlanModeRequest>);
    let is_permission_mode = use_signal(|| false);

    UsePermissionsReturn {
        allowed_tools,
        permission_request,
        plan_mode_request,
        is_permission_mode,
    }
}
