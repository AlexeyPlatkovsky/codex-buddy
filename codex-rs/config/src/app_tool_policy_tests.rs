use std::collections::BTreeMap;
use std::collections::HashMap;

use super::*;
use crate::AppRequirementToml;
use crate::AppToolRequirementToml;
use crate::AppToolsRequirementsToml;
use crate::types::AppConfig;
use crate::types::AppToolConfig;
use crate::types::AppToolsConfig;

#[test]
fn evaluator_applies_managed_approval_before_local_tool_policy() {
    let apps_config = AppsConfigToml {
        default: None,
        apps: HashMap::from([(
            "calendar".to_string(),
            AppConfig {
                enabled: true,
                tools: Some(AppToolsConfig {
                    tools: HashMap::from([(
                        "events/create".to_string(),
                        AppToolConfig {
                            enabled: Some(true),
                            approval_mode: Some(AppToolApproval::Prompt),
                        },
                    )]),
                }),
                ..Default::default()
            },
        )]),
    };
    let requirements = AppsRequirementsToml {
        apps: BTreeMap::from([(
            "calendar".to_string(),
            AppRequirementToml {
                enabled: None,
                tools: Some(AppToolsRequirementsToml {
                    tools: BTreeMap::from([(
                        "events/create".to_string(),
                        AppToolRequirementToml {
                            approval_mode: Some(AppToolApproval::Approve),
                        },
                    )]),
                }),
            },
        )]),
    };

    assert_eq!(
        AppToolPolicyEvaluator::from_parts(Some(apps_config), Some(&requirements)).policy(
            AppToolPolicyInput {
                connector_id: Some("calendar"),
                tool_name: "events/create",
                tool_title: None,
                destructive_hint: Some(true),
                open_world_hint: Some(true),
            },
        ),
        AppToolPolicy {
            enabled: true,
            approval: AppToolApproval::Approve,
        }
    );
}

#[test]
fn evaluator_applies_managed_disable_without_local_apps_config() {
    let requirements = AppsRequirementsToml {
        apps: BTreeMap::from([(
            "calendar".to_string(),
            AppRequirementToml {
                enabled: Some(false),
                tools: None,
            },
        )]),
    };

    assert!(!AppToolPolicyEvaluator::from_parts(None, Some(&requirements)).app_enabled("calendar"));
}
