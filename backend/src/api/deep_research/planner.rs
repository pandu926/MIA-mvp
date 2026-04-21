use crate::config::DeepResearchProvider;

const STAGE_PLAN: &str = "plan";
const STAGE_GATHER_INTERNAL: &str = "gather_internal";
const STAGE_GATHER_EXTERNAL: &str = "gather_external";
const STAGE_SYNTHESIZE: &str = "synthesize";
const STAGE_FINALIZE: &str = "finalize";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolRequirement {
    Required,
    Optional,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannedTool {
    pub name: &'static str,
    pub requirement: ToolRequirement,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannedStep {
    pub step_key: &'static str,
    pub title: &'static str,
    pub agent_name: &'static str,
    pub tools: Vec<PlannedTool>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResearchPlan {
    pub planner_version: &'static str,
    pub execution_mode: &'static str,
    pub steps: Vec<PlannedStep>,
}

pub(crate) fn build_research_plan(provider: DeepResearchProvider) -> ResearchPlan {
    let external_tools = match provider {
        DeepResearchProvider::HeuristMeshX402 => vec![PlannedTool {
            name: "get_optional_narrative_context",
            requirement: ToolRequirement::Optional,
        }],
        DeepResearchProvider::NativeXApi => vec![PlannedTool {
            name: "get_provider_health",
            requirement: ToolRequirement::Optional,
        }],
    };

    ResearchPlan {
        planner_version: "v0_phase2_internal",
        execution_mode: "sequential",
        steps: vec![
            PlannedStep {
                step_key: STAGE_PLAN,
                title: "Plan the research run",
                agent_name: "planner_agent",
                tools: Vec::new(),
            },
            PlannedStep {
                step_key: STAGE_GATHER_INTERNAL,
                title: "Gather internal launch signals",
                agent_name: "market_wallet_agent",
                tools: vec![
                    PlannedTool {
                        name: "get_market_structure",
                        requirement: ToolRequirement::Required,
                    },
                    PlannedTool {
                        name: "get_wallet_structure",
                        requirement: ToolRequirement::Required,
                    },
                    PlannedTool {
                        name: "get_deployer_memory",
                        requirement: ToolRequirement::Required,
                    },
                    PlannedTool {
                        name: "get_linked_launch_cluster",
                        requirement: ToolRequirement::Required,
                    },
                    PlannedTool {
                        name: "get_pattern_matches",
                        requirement: ToolRequirement::Required,
                    },
                ],
            },
            PlannedStep {
                step_key: STAGE_GATHER_EXTERNAL,
                title: "Attach optional external enrichment",
                agent_name: "narrative_agent",
                tools: external_tools,
            },
            PlannedStep {
                step_key: STAGE_SYNTHESIZE,
                title: "Assemble the dossier",
                agent_name: "synthesis_agent",
                tools: vec![PlannedTool {
                    name: "build_premium_dossier",
                    requirement: ToolRequirement::Required,
                }],
            },
            PlannedStep {
                step_key: STAGE_FINALIZE,
                title: "Finalize report artifacts",
                agent_name: "run_finalizer",
                tools: vec![PlannedTool {
                    name: "persist_deep_research_report",
                    requirement: ToolRequirement::Required,
                }],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{build_research_plan, ToolRequirement};
    use crate::config::DeepResearchProvider;

    #[test]
    fn heurist_plan_keeps_optional_narrative_tool() {
        let plan = build_research_plan(DeepResearchProvider::HeuristMeshX402);
        let external = plan
            .steps
            .iter()
            .find(|step| step.step_key == "gather_external")
            .expect("external step");

        assert_eq!(plan.planner_version, "v0_phase2_internal");
        assert_eq!(external.tools.len(), 1);
        assert_eq!(external.tools[0].name, "get_optional_narrative_context");
        assert_eq!(external.tools[0].requirement, ToolRequirement::Optional);
    }

    #[test]
    fn internal_step_keeps_five_required_tools() {
        let plan = build_research_plan(DeepResearchProvider::NativeXApi);
        let internal = plan
            .steps
            .iter()
            .find(|step| step.step_key == "gather_internal")
            .expect("internal step");

        assert_eq!(internal.tools.len(), 5);
        assert!(internal
            .tools
            .iter()
            .all(|tool| tool.requirement == ToolRequirement::Required));
        assert!(internal
            .tools
            .iter()
            .any(|tool| tool.name == "get_pattern_matches"));
    }
}
