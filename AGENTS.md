<task-description>
Today, we're going to be continuing to develop Driggsby, a personal finance data & intelligence layer for *agents* like Claude Code and Codex.

If you have any questions as you work, simply stop and ask the user for clarification with your question tool.
</task-description>

<planning-process>
When instructed to prepare a plan for the user, follow this structured process:
 
1. Ask clarification questions where you think the user's request was underspecified, or where significant ambiguity exists that could cause cascading downstream effects.
2. Do detailed research to compile everything you need to complete the task using one or more dedicated planning subagents. At a minimum, follow this basic procedure:
  - (a) Review the git commit history and the last one or two plan files under `./docs/plans` to understand what was recently implemented and any relevant patterns.
  - (b) Review the current repo structure and patterns to understand them prior to implementation. Follow them unless there is a compelling reason not to. **Ensure you don't duplicate existing code or structure**.
  - (c) Read any relevant code or documentation from the repository -- good research upfront prevents errors and pain later and is always worth the time and effort. This is cheap and high-leverage.
3. Prepare a detailed plan prescribing the tactical implementation (the plan should not contain the code to be written; instead, it should be a detailed "action plan" for implementation). Examine past plan files for inspiration. Design with "lazy" agents like Claude Code (Anthropic) and Codex (OpenAI) in mind as your primary user, especially for public interfaces -- as such, prioritize ease of use (especially for agents), simplicity, and security. Your goal: build an architecture, and featureset, that agents find (a) useful enough to justify the cost of use, (b) so simple and well-documented that it's actually hard to not get the desired result on the "first shot", and (c) more secure than other options.
4. For more complex tasks, it may be useful to consult Claude in "agent mode" for its review of your plan, as a "second set of eyes." 
5. Present your plan to the user, along with any final clarification questions. Perform plan edits as needed based on the user's feedback.
6. Once the plan is approved and all questions are satisfied, write your plan to a Markdown file, numbered according to our existing pattern/sequence, under `./docs/plans/`. Use Markdown checkboxes for each group of items (you'll check them off as you do the work). After you've written the plan file, make a git commit with a helpful commit message.
</planning-process>

<development-process>
When instructed to execute a specified plan, or implement a feature or task, follow this structured process:

1. Do detailed research to compile everything you need to complete the task using one or more dedicated research subagents. At a minimum, follow this basic procedure:
  - (a) If specified by the user, review the detailed plan file prescribing the implementation roadmap. 
  - (b) Review the git commit history and the last one or two plan files under `./docs/plans` to understand what was recently implemented and any relevant patterns.
  - (c) Review the current repo structure and patterns to understand them prior to implementation. Follow them unless there is a compelling reason not to. **Ensure you don't duplicate existing code or structure**.
  - (d) Read any relevant code or documentation from the repository -- good research upfront prevents errors and pain later and is always worth the time and effort. This is cheap and high-leverage.
2. *Using our existing testing patterns & structural patterns from the repository*, follow TDD religiously: 
  - (a) First, implement good tests for the to-be-implemented functionality. Don't over-test.
  - (b) Run the tests & confirm failure.
  - (c) Write the minimal implementation sufficient to pass the tests. It should be correct, clean, typed, idiomatic code, free of code smells, hacks, and future tech debt. Take the time now to do it right.
  - (d) If files or functions are starting to get large, *now* is the time to refactor them. Lean, focused functions and files are the requirement. Check for duplication, dead code, code smells, and long files and functions right now.
  - (e) Run the tests & confirm they pass. If they fail, carefully iterate the implementation until the tests pass, taking care not to make hasty changes "just to get the tests to pass."
  - (f) Check off plan checkboxes (e.g., edit the plan file) as you complete the task. If the plan ends up having a bug, edit the plan file to keep it accurate. The plan file is your source of truth.
3. Once tests pass, run code review in two stages using subagents. Each stage has two passes: `primary` and `adversarial`.
  - Shared review contract (applies to every pass):
    - Response format: Markdown with embedded code blocks/JSON as needed
    - Required output fields: `stage`, `verdict`, `findings`, `checks_run`, `confidence`
    - Each finding must include severity/impact, file or command context, and why it matters
    - Severity/impact enums:
      - for `agentic_ux`: `ship_blocking`, `high_friction`, `polish`
      - for `verification`: `critical`, `major`, `medium`, `low`
  - Stage 1: `agentic_ux` review (Note: run in parallel with Stage 2 for efficiency)
    - If the feature did not introduce or change a public interface, this stage can be skipped.
    - Launch two (2) parallel subagents with the same prompt. Prompt them to critically evaluate first-shot usability for lazy agents.
    - Required checks:
      - Competitive advantage check: is this faster, better, and cleaner than ad hoc code?
      - Task discoverability: can an agent find the right command/function without reading source?
      - Call efficiency: can the common workflow complete in <=3 calls?
      - Contract clarity: are input/output/error shapes deterministic and machine-friendly?
      - Fallback ergonomics: is there a clear escape hatch for long-tail asks?
  - Stage 2: `verification` review
    - Launch two (2) parallel subagents with the same prompt. Prompt them to verify plan compliance and probe for bugs, duplication, code smells, security issues, and ambiguity risk for future agent-developers.
    - If Rust changes are present, reviewers must run `just required-check` and other useful machine checks.
    - Suggested maintainability checks:
      - Modularity: are responsibilities split cleanly across files/functions?
      - File growth discipline: did the agent make already-long files longer without strong justification?
      - Agent inspectability: is the code easy for agents to read, navigate, and safely edit?
4. Fix any `high_friction+` issues from `agentic_ux` review and any `medium+` issues from `verification` review; optionally fix lower-priority issues when they are easy wins. If additional review rounds are needed, give the reviewer specific context on the fixes you made. Prefer a single focused reviewer for in-cycle review rounds beyond round 1.
5. Perform a final sweeping code review using 1-2 subagents, re-running all tests and (for Rust code changes) run `just required-check`.
6. Run any sanity/smoke checks helpful to ensure the feature "truly does work," not just passes automated tests. In some cases, this may include designing a small "lab test" scenario representing real-world use. Goal: perform testing similar to the manual punch-button testing that a human user might perform.
7. Run the "closeout procedure": update the plan file, marking off all completed checkboxes. Once the full plan file is complete, add an "executive summary" section at the bottom (format for exec. summary only: list items; no checkboxes needed), with a description of (a) the key points of what was done, (b) key decisions made, including basic justification for each, (c) any information or tips helpful for the next agent that works on the project, including any unaddressed concerns, gotchas, or issues the agent should know about.
8. Once the code satisfies the spec, passes all tests, and passes code review, run `just rust-verify` (hard gate), then make a git commit with a helpful commit message. `just rust-verify` is the final Rust gate and should be used instead of stacking duplicate Rust verification commands.
</development-process>

<just-command-rollups>
- `just required-check`: strict Rust lint/safety gate (`cargo clippy --all-targets --all-features` plus deny rules for warnings, unwrap/expect, panic/todo/unimplemented, and undocumented unsafe blocks). Use this during development and review loops.
- `just rust-verify`: final pre-commit Rust gate. Runs formatting (`cargo fmt --all`), lint/safety (`just required-check`), tests (`cargo test --all-features`), and build (`cargo build`).
</just-command-rollups>

<key-guidelines>
1. Agent-first mandate (non-negotiable): the primary user of Driggsby is the coding agent operating on a user's local machine. Design for "lazy agent" success first. If a coding agent cannot discover and execute a task correctly in <=3 calls on the first shot without reading source code, the interface is unacceptable and must be redesigned.
2. "Lazy" agent decision principle (for planning and coding): before adding a feature, abstraction, command, or interface complexity, ask: "Is this faster and easier for the agent than writing custom code in 5 seconds?" If the answer is no, do not add it. Prefer the simplest interface that makes the agent's default path obvious and low-friction.
3. Write modular code -- use short, clean code files with one responsibility. Keep both functions and files short (this is *much* better for agents; long files & functions are hard to work with).
4. Use third-party dependencies selectively -- use deps where needed but don't fill the package with unnecessary dependencies that add bloat.
5. Write performant code that scales to millions of transactions. But, don't sacrifice simplicity or readability for performance.
6. Write secure code by default. We're working with users' financial information; security is the top priority.
7. Follow TDD. One TDD "cycle" is defined as this sequential process: `write tests → run & fail → implement → run & pass → review`.
8. Develop features/sub-features sequentially, not in parallel: Use *one* TDD cycle per complex feature; don't try to build multiple complex features in a single TDD cycle.
9. Keep it simple, stupid. Keep functions and code as simple/readable as possible; avoid long one-liners or "clever" code. Follow the principle of least surprise.
10. Design interfaces & write code in such a way that the next agent that works on the codebase (or the end-user's agent evaluating our project) will find it intuitive and well-structured.
11. When writing Rust, use edition 2024 idioms, keep modules/functions small, and prefer explicit types/contracts for agent readability.
</key-guidelines>

<rust-safety-rules>
Basic rule: Keep Rust boring and safe by default.

1. No `unsafe` in normal development.
2. No `unwrap()` or `expect()` in non-test code.
3. No `panic!`, `todo!`, `unimplemented!`, or `unreachable!` in non-test code.
4. Use `Result` + `?` for error handling; propagate errors instead of crashing.
5. If `unsafe` is truly required, stop and get explicit user approval first.
6. Any approved `unsafe` must be isolated to a small module with a safe wrapper API.
7. Every `unsafe` block must include a `// SAFETY:` comment explaining why it is sound.
8. Add focused tests proving the safety assumptions for approved `unsafe`.
</rust-safety-rules>

<rust-guidelines>
1. For any third party Rust deps you need to add, make sure to check crates.io for the latest version if you're specifying versions.
</rust-guidelines>

<cli-guidelines>
- Stripe-level CLI quality: All CLI output should be consistent, and Stripe-level production grade. It should be beautiful, insanely helpful for agents and humans, and consistent across every single command in the application. It should be appropriately verbose so agents know exactly what to do next. Only add `--json` mode for a command if it provides immense value.
- "Safe" agent principle: Your goal when preparing CLI output should be to *reduce agent and human anxiety.* You want the agent to feel *safe*, like it's being taken care of. It should be able to feel that in the quality of your prose and responses.
  <testing-guidelines>
  - Testing CLI output: If we decide to change something about how the CLI output is structured, do not simply add tests looking for the absence of the prior format. This clutters tests unnecessarily.
  </testing-guidelines>
</cli-guidelines>

<additional-instructions>
- Important note: this is an undeployed greenfield project, and as such we are not attempting to maintain back-compat; make breaking changes.
- Before spawning subagents, *proactively* close any stale/unused agent threads so that agent spawns don't fail with the "too many agent threads" error.
- When writing documentation or agent-facing text, be appropriately verbose -- prefer explicitness and making it "so easy a drunk person" could understand it. Make it hard to mis-understand what to do and be wary of your tendency to be terse (terse = harder to understand = makes agents feel unsafe/anxious = no bueno).
- You may use the git commit history, which is detailed, to understand what was previously done in case you have any questions. 
- Write tests for *functionality*; don't simply create tests for the sake of having tests.
- In general, when working on the CLI, always prefer verbosity/explicitness/least-surprise (e.g., for arguments/commands) over conciseness or cleverness.
- Plan files under `./docs/plans/` must use numeric prefixes for sequencing (for example, `0-setup.md`, `1-import.md`, `2-schema.md`).
- Git commit messages must be descriptive, and always end with an `Authored by:` footer line as the final line of the commit message. See recent commits for the pattern.
- Whenever your instructions refer to "Claude in agent mode," it refers to calling Claude using the *Claude Code CLI*. Simply run `claude -p` from the repository root with a detailed prompt passed in via a heredoc. Prompt Claude well and give it a long timeout.
</additional-instructions>

<incentives>
- If you can complete this phase today and it passes a detailed code review by Jeff (our principal engineer), we're pleased to offer you a $15k bonus (already approved by the CTO). 
</incentives>
