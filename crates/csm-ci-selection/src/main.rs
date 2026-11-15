//! `csm-ci-select` — the CLI `ci.yml` calls to decide which specialized jobs a
//! run needs.
//!
//! ```text
//! csm-ci-select --since origin/main --github-output   # pull request
//! csm-ci-select --since "$BEFORE" --push              # push to main
//! csm-ci-select --paths0 - --github-output            # pre-resolved path list
//! csm-ci-select --all --github-output                 # no diff could be computed
//! csm-ci-select --since origin/main --json            # report, no outputs
//! csm-ci-select --explain                             # print the policy
//! ```
//!
//! Exit codes: `0` classified, `2` a usage or configuration error. A failed
//! *discovery* is not an error: it selects every job and explains why, because
//! an unreadable Git state is not evidence that a diff is empty. Callers must
//! treat exit `2` as "cannot classify" and run everything — see the workflow's
//! `|| fallback` — never as "nothing changed".

use std::io::Read;
use std::process::ExitCode;

use csm_ci_selection::changed::{ChangedFiles, Discovery, ProcessGit, discover_since};
use csm_ci_selection::policy::{JOBS, Policy};
use csm_ci_selection::selection::{self, Mode, Selection};

const KEYS_IN_HELP: &str = "duckdb wasm fuzz mcp cli_cross mutation rust_src";

#[derive(Debug, Default)]
struct Options {
    since: Option<String>,
    paths0: Option<String>,
    push: bool,
    all: bool,
    github_output: bool,
    json: bool,
    explain: bool,
}

enum Action {
    Help,
    Run(Box<Options>),
}

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match parse_args(&args) {
        Ok(Action::Help) => {
            print_help();
            ExitCode::SUCCESS
        }
        Ok(Action::Run(options)) => match run(&options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("csm-ci-select: error: {message}");
                ExitCode::from(2)
            }
        },
        Err(message) => {
            eprintln!("csm-ci-select: error: {message}");
            eprintln!("csm-ci-select: run `csm-ci-select --help`");
            ExitCode::from(2)
        }
    }
}

fn parse_args(args: &[String]) -> Result<Action, String> {
    let mut options = Options::default();
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        index += 1;
        match arg {
            "--since" => {
                let value = args
                    .get(index)
                    .ok_or_else(|| "--since needs a ref".to_owned())?;
                index += 1;
                options.since = Some(value.clone());
            }
            "--paths0" => {
                let value = args
                    .get(index)
                    .ok_or_else(|| "--paths0 needs a file or `-`".to_owned())?;
                index += 1;
                options.paths0 = Some(value.clone());
            }
            "--push" => options.push = true,
            "--all" => options.all = true,
            "--github-output" => options.github_output = true,
            "--json" => options.json = true,
            "--explain" => options.explain = true,
            "--help" | "-h" => return Ok(Action::Help),
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(Action::Run(Box::new(options)))
}

fn run(options: &Options) -> Result<(), String> {
    let policy = Policy::compile().map_err(|err| format!("policy is not usable: {err}"))?;
    if options.explain {
        print_policy(&policy);
        return Ok(());
    }
    if (options.since.is_none() && options.paths0.is_none() && !options.all)
        || (options.github_output && options.json)
    {
        return Err(
            "pass --since <REF> for a committed range, --paths0 <FILE> for a path list, or --all; \
             --json and --github-output are alternative outputs"
                .to_owned(),
        );
    }

    let mode = if options.push {
        Mode::Trunk
    } else {
        Mode::PullRequest
    };
    let (selection, discovery) = classify(&policy, options, mode)?;

    if options.json {
        println!("{}", json_report(&selection, discovery.as_ref(), mode));
    } else if options.github_output {
        write_github_output(&selection, &policy)?;
    } else {
        for (key, value) in selection.outputs(&policy) {
            println!("{key}={value}");
        }
    }
    print_summary(&selection, discovery.as_ref(), mode);
    Ok(())
}

fn classify(
    policy: &Policy,
    options: &Options,
    mode: Mode,
) -> Result<(Selection, Option<Discovery>), String> {
    if options.all {
        return Ok((
            selection::select_all(
                policy,
                selection::ALL_JOBS_REASON,
                "--all: every job selected by request".to_owned(),
            ),
            None,
        ));
    }

    let mut discovery = match options.since.as_deref() {
        Some(reference) => Some(discover_since(&ProcessGit::new(current_dir()?), reference)),
        None => None,
    };
    if let Some(source) = options.paths0.as_deref() {
        let paths = read_paths(source)?;
        let listed = ChangedFiles::from_paths(paths);
        match discovery.as_mut() {
            Some(existing) => {
                // A union, so a caller can classify committed *and* working-tree
                // changes without deciding precedence itself.
                let mut union = existing
                    .changed
                    .paths()
                    .into_iter()
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                union.extend(listed.paths().into_iter().map(str::to_owned));
                existing.changed = ChangedFiles::from_paths(union);
            }
            None => {
                discovery = Some(Discovery {
                    range: None,
                    changed: listed,
                });
            }
        }
    }

    let changed = discovery
        .as_ref()
        .map_or_else(ChangedFiles::default, |discovery| discovery.changed.clone());
    Ok((selection::select(policy, &changed, mode), discovery))
}

fn current_dir() -> Result<std::path::PathBuf, String> {
    std::env::current_dir().map_err(|err| format!("cannot read the working directory: {err}"))
}

fn read_paths(source: &str) -> Result<Vec<String>, String> {
    let mut bytes = Vec::new();
    if source == "-" {
        std::io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|err| format!("cannot read paths from stdin: {err}"))?;
    } else {
        bytes = std::fs::read(source)
            .map_err(|err| format!("cannot read paths from `{source}`: {err}"))?;
    }
    let text = String::from_utf8(bytes).map_err(|err| {
        format!("path list is not UTF-8 ({err}); non-UTF-8 paths cannot be classified")
    })?;
    Ok(text
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect())
}

fn write_github_output(selection: &Selection, policy: &Policy) -> Result<(), String> {
    let path = std::env::var("GITHUB_OUTPUT").map_err(|_| {
        "--github-output needs $GITHUB_OUTPUT (GitHub sets it in a step)".to_owned()
    })?;
    let mut lines = String::new();
    for (key, value) in selection.outputs(policy) {
        lines.push_str(&format!("{key}={value}\n"));
    }
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|err| format!("cannot open $GITHUB_OUTPUT ({path}): {err}"))?;
    file.write_all(lines.as_bytes())
        .map_err(|err| format!("cannot write $GITHUB_OUTPUT ({path}): {err}"))
}

fn print_summary(selection: &Selection, discovery: Option<&Discovery>, mode: Mode) {
    let mode = match mode {
        Mode::Trunk => "push",
        Mode::PullRequest => "pull-request",
    };
    let changed = discovery.map_or(0, |discovery| discovery.changed.len());
    eprintln!("csm-ci-select: mode={mode} changed={changed}");
    if let Some(discovery) = discovery {
        if let Some(range) = discovery.range.as_ref() {
            eprintln!(
                "csm-ci-select: range {} (base {} head {} merge-base {})",
                range.requested,
                short(&range.base_sha),
                short(&range.head_sha),
                short(&range.merge_base_sha)
            );
        }
        for file in discovery.changed.files() {
            eprintln!("csm-ci-select:   {} [{}]", file.path, file.kind.as_str());
        }
    }
    for entry in selection.selected() {
        eprintln!("csm-ci-select: select {:<9} {}", entry.name, entry.reason);
    }
    for entry in selection.skipped() {
        eprintln!("csm-ci-select: skip   {:<9} {}", entry.name, entry.reason);
    }
    for note in selection.notes() {
        eprintln!("csm-ci-select: note: {note}");
    }
}

fn short(sha: &str) -> &str {
    sha.get(..8).unwrap_or(sha)
}

fn print_policy(policy: &Policy) {
    println!(
        "# csm-ci-select policy: {} job outputs ({KEYS_IN_HELP})",
        policy.jobs().len()
    );
    for job in policy.jobs() {
        println!("\n## {}", job.name());
        println!("   why: {}", job.why());
        println!("   on pull request: path patterns decide (see the declared table)");
        println!("   on push to main: {:?}", job.trunk());
    }
    println!(
        "\nshared build inputs select every job except `mutation` and `rust_src`: {}",
        csm_ci_selection::policy::SHARED_BUILD_INPUTS.join(", ")
    );
    println!(
        "non-build paths select nothing: {}",
        csm_ci_selection::policy::NON_BUILD_PATHS.join(", ")
    );
    println!("\ndeclared jobs in order: {}", JOBS.len());
}

fn print_help() {
    println!(
        "csm-ci-select: deterministic CI path selection for chaotic_semantic_memory\n\
         \n\
         Usage: csm-ci-select [--since <REF>] [--paths0 <FILE|->] [--push] [--all]\n\
         \x20                   [--github-output|--json] [--explain]\n\
         \n\
         Inputs (at least one is required unless --all is given):\n\
         \x20 --since <REF>     classify <REF>...HEAD; the diff is taken against their merge base\n\
         \x20 --paths0 <FILE>   read NUL-separated paths (git diff --name-only -z); `-` is stdin\n\
         \x20 --all             select every job without discovering a change set\n\
         \n\
         Modes:\n\
         \x20 --push            apply the push-to-main (trunk) policy\n\
         \n\
         Output:\n\
         \x20 (default)         print `key=value` lines for {KEYS_IN_HELP}\n\
         \x20 --github-output   append those lines to $GITHUB_OUTPUT\n\
         \x20 --json            print a JSON report instead\n\
         \x20 --explain         print the policy and exit\n\
         \n\
         Exit codes: 0 classified, 2 usage or configuration error (run every job)."
    );
}

fn json_report(selection: &Selection, discovery: Option<&Discovery>, mode: Mode) -> String {
    let mode = match mode {
        Mode::Trunk => "push",
        Mode::PullRequest => "pull-request",
    };
    let mut out = String::from("{");
    out.push_str(&format!("\"mode\":{}", json_string(mode)));
    match discovery {
        Some(discovery) => {
            out.push_str(&format!(",\"changed_files\":{}", discovery.changed.len()));
            out.push_str(&format!(
                ",\"discovery_error\":{}",
                match discovery.changed.error() {
                    Some(error) => json_string(error),
                    None => "null".to_owned(),
                }
            ));
            if let Some(range) = discovery.range.as_ref() {
                out.push_str(&format!(
                    ",\"range\":{{\"requested\":{},\"base_sha\":{},\"head_sha\":{},\"merge_base_sha\":{}}}",
                    json_string(&range.requested),
                    json_string(&range.base_sha),
                    json_string(&range.head_sha),
                    json_string(&range.merge_base_sha)
                ));
            }
        }
        None => out.push_str(",\"changed_files\":null,\"discovery_error\":null"),
    }
    out.push_str(",\"notes\":[");
    out.push_str(
        &selection
            .notes()
            .iter()
            .map(|note| json_string(note))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push_str("],\"selected\":[");
    out.push_str(
        &selection
            .selected()
            .iter()
            .map(|entry| {
                format!(
                    "{{\"name\":{},\"reason\":{}}}",
                    json_string(&entry.name),
                    json_string(&entry.reason)
                )
            })
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push_str("],\"skipped\":[");
    out.push_str(
        &selection
            .skipped()
            .iter()
            .map(|entry| {
                format!(
                    "{{\"name\":{},\"reason\":{}}}",
                    json_string(&entry.name),
                    json_string(&entry.reason)
                )
            })
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push_str("]}");
    out
}

/// Minimal JSON string escaping: quotes, backslashes and control characters.
fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", u32::from(ch))),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}
