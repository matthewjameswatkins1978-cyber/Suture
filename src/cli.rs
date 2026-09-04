use clap::{Args, Parser, Subcommand, ValueEnum, ValueHint};

pub const THREADMOTH_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(
    name = "threadmoth",
    version = THREADMOTH_VERSION,
    about = "Fast, deterministic structural search and rewrite for AI agents.",
    long_about = "Threadmoth changes exactly the state a request authorizes, refuses ambiguity, and returns a certificate describing what was observed and what actually changed.",
    after_help = "Start with: threadmoth suggest PATH\nInspect capabilities: threadmoth capabilities\nGenerate shell completion: threadmoth completions <shell>\nRun checks: threadmoth benchmark --tough",
    propagate_version = true,
    disable_help_subcommand = true,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Apply one verified mutation.
    #[command(alias = "apply")]
    Mutate(RequestArgs),

    /// Preview a mutation without writing.
    #[command(alias = "dry-run")]
    Preview(RequestArgs),

    /// Apply or preview a guarded multi-file transaction.
    Transact(TransactionArgs),

    /// Legacy alias for `transact --preview`.
    #[command(hide = true)]
    TransactionPreview(RequestArgs),

    /// Recover interrupted transaction state where possible.
    Recover,

    /// Show machine-readable capabilities, optionally for one file.
    Capabilities(CapabilitiesArgs),

    /// Show request examples.
    Examples {
        /// Optional example topic.
        topic: Option<String>,
    },

    /// Run correctness-checked performance and safety checks.
    Benchmark(BenchmarkArgs),

    /// Legacy alias for `benchmark --torture`.
    #[command(hide = true)]
    Torture {
        /// Emit machine-readable JSON.
        #[arg(short = 'j', long)]
        json: bool,
    },

    /// Search or show detailed command help.
    Help(HelpArgs),

    /// Explain a refusal or failure reason code.
    Explain {
        /// Reason code such as TARGET_AMBIGUOUS.
        code: String,
        /// Emit machine-readable JSON.
        #[arg(short = 'j', long)]
        json: bool,
    },

    /// Suggest a safe request shape for a workspace file or refusal.
    Suggest(SuggestArgs),

    /// Inspect file identity, encoding and newline facts.
    Inspect {
        /// Workspace-relative file path.
        #[arg(value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
    },

    /// Print protocol or request schemas.
    Schema(SchemaArgs),

    /// Check runtime and CLI installation health.
    Doctor,

    /// Generate shell completion for Threadmoth.
    Completions {
        /// Shell to generate completion for.
        #[arg(value_enum)]
        shell: CompletionShell,
    },

    /// Generate a roff man page to stdout or a file.
    Manpage {
        /// Optional output path. Defaults to stdout.
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        output: Option<std::path::PathBuf>,
    },

    /// Run the MCP stdio server.
    Mcp,
}

#[derive(Args, Debug)]
pub struct RequestArgs {
    /// Read the JSON request from a file instead of stdin.
    #[arg(short = 'r', long, value_hint = ValueHint::FilePath)]
    pub request: Option<std::path::PathBuf>,
}

#[derive(Args, Debug)]
pub struct TransactionArgs {
    /// Read the transaction JSON request from a file instead of stdin.
    #[arg(short = 'r', long, value_hint = ValueHint::FilePath)]
    pub request: Option<std::path::PathBuf>,

    /// Preview the transaction without committing it.
    #[arg(short = 'n', long)]
    pub preview: bool,
}

#[derive(Args, Debug)]
pub struct CapabilitiesArgs {
    /// Optional capability/provider selector.
    pub selector: Option<String>,

    /// Evaluate capabilities for this workspace-relative file.
    #[arg(long = "for", value_hint = ValueHint::FilePath)]
    pub for_path: Option<std::path::PathBuf>,

    /// Emit compact machine-readable JSON.
    #[arg(short = 'j', long)]
    pub json: bool,

    /// Force pretty JSON output.
    #[arg(long)]
    pub pretty: bool,

    /// Compatibility flag accepted by existing automation.
    #[arg(long)]
    pub all: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum BenchmarkProfile {
    Quick,
    Standard,
    Tough,
}

#[derive(Args, Debug)]
pub struct BenchmarkArgs {
    /// Legacy positional profile: quick, standard or tough.
    #[arg(value_enum, hide = true)]
    pub profile: Option<BenchmarkProfile>,

    /// Run the quick benchmark profile.
    #[arg(short = 'q', long, conflicts_with_all = ["tough", "torture"])]
    pub quick: bool,

    /// Run the tough benchmark profile.
    #[arg(short = 't', long, conflicts_with_all = ["quick", "torture"])]
    pub tough: bool,

    /// Run the deterministic safety torture suite instead of timing benchmarks.
    #[arg(short = 'x', long, conflicts_with_all = ["quick", "tough", "profile"])]
    pub torture: bool,

    /// Emit machine-readable JSON.
    #[arg(short = 'j', long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct HelpArgs {
    /// Command to show detailed help for.
    pub command: Option<String>,

    /// Search Threadmoth help metadata.
    #[arg(long)]
    pub find: Option<String>,
}

#[derive(Args, Debug)]
pub struct SuggestArgs {
    /// Workspace-relative file to inspect.
    #[arg(value_hint = ValueHint::FilePath, required_unless_present = "from_refusal")]
    pub path: Option<std::path::PathBuf>,

    /// Read an existing refusal certificate from this file, or `-` for stdin.
    #[arg(long, value_hint = ValueHint::FilePath, conflicts_with = "path")]
    pub from_refusal: Option<String>,

    /// High-level desired operation, such as set-value.
    #[arg(long)]
    pub goal: Option<String>,

    /// Structural location, such as package.name.
    #[arg(long)]
    pub at: Option<String>,

    /// Suggestion mode. Defaults to safe.
    #[arg(long, default_value = "safe")]
    pub mode: String,
}

#[derive(Args, Debug)]
pub struct SchemaArgs {
    /// Optional schema scope, for example request.
    pub scope: Option<String>,

    /// Emit compact machine-readable JSON.
    #[arg(short = 'j', long)]
    pub json: bool,

    /// Force pretty JSON output.
    #[arg(long)]
    pub pretty: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    Powershell,
}

impl From<CompletionShell> for clap_complete::Shell {
    fn from(value: CompletionShell) -> Self {
        match value {
            CompletionShell::Bash => clap_complete::Shell::Bash,
            CompletionShell::Zsh => clap_complete::Shell::Zsh,
            CompletionShell::Fish => clap_complete::Shell::Fish,
            CompletionShell::Powershell => clap_complete::Shell::PowerShell,
        }
    }
}
