use crate::ToolName;

pub fn code_mode_name_for_tool_name(tool_name: &ToolName) -> String {
    if tool_name.is_default_namespace() {
        return tool_name.name.clone();
    }

    match tool_name.namespace.as_deref() {
        Some(namespace) if namespace.ends_with('_') || tool_name.name.starts_with('_') => {
            format!("{namespace}{}", tool_name.name)
        }
        Some(namespace) => format!("{namespace}__{}", tool_name.name),
        None => tool_name.name.clone(),
    }
}
