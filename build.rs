use std::env;

fn main() {
    let flavor = env_value_or("THREADMOTH_BUILD_FLAVOR", "portable");
    let baseline = env_value_or("THREADMOTH_CPU_BASELINE", "default");
    let pgo = env_value_or("THREADMOTH_PGO_USED", "false");
    let profile = env::var("THREADMOTH_OPTIMIZATION_PROFILE")
        .or_else(|_| env::var("PROFILE"))
        .unwrap_or_else(|_| "dev".into());
    let target = env_value_or("TARGET", "unknown");
    println!("cargo:rustc-env=THREADMOTH_BUILD_FLAVOR={flavor}");
    println!("cargo:rustc-env=THREADMOTH_CPU_BASELINE={baseline}");
    println!("cargo:rustc-env=THREADMOTH_PGO_USED={pgo}");
    println!("cargo:rustc-env=THREADMOTH_OPTIMIZATION_PROFILE={profile}");
    println!("cargo:rustc-env=THREADMOTH_TARGET_TRIPLE={target}");
}

fn env_value_or(name: &str, fallback: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.into())
}
