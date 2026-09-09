#![forbid(unsafe_code)]

use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct BuildInfo {
    pub build_flavor: &'static str,
    pub target_triple: &'static str,
    pub cpu_baseline: &'static str,
    pub optimization_profile: &'static str,
    pub pgo_used: bool,
}

pub fn current() -> BuildInfo {
    BuildInfo {
        build_flavor: env!("THREADMOTH_BUILD_FLAVOR"),
        target_triple: env!("THREADMOTH_TARGET_TRIPLE"),
        cpu_baseline: env!("THREADMOTH_CPU_BASELINE"),
        optimization_profile: env!("THREADMOTH_OPTIMIZATION_PROFILE"),
        pgo_used: matches!(env!("THREADMOTH_PGO_USED"), "1" | "true" | "yes"),
    }
}
