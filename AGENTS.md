
<task-description>
Today, we're going to be continuing to develop Driggsby, a personal finance data & intelligence layer for agents.

If you have any questions as you work, simply stop and ask the user for clarification.
</task-description>

As you work, you'll follow this process:

<development-process>
1. Do detailed research to compile everything you need to complete the task using a dedicated planning subagent. Perform the detailed research necessary to perform the actual implementation, to include: (a) review the current repo structure and patterns to understand them prior to implementation. Follow them unless there is a compelling reason not to -- **ENSURE YOU DO NOT DUPLICATE EXISTING CODE OR STRUCTURE**, (b) read any relevant documentation from the repository - some good research upfront prevents runtime errors and pain later and is always worth the time and effort, and (c) prepare a detailed plan prescribing the tactical implementation plan (the plan doesn't need to contain all the code to be written, just be a detailed "action plan" for how to create the code and what it needs to do). 

**Once your plan file is presented to and approved by the user**,

2. Write your plan to a Markdown file, numbered according to our existing pattern/sequence, under `plans/`. Use Markdown checkboxes for each group of items, and check it off as you go.

3. Review the git commit history to understand what was recently implemented and any relevant patterns.

4. *Using our existing testing patterns & structural patterns from the repository*, follow TDD religiously: 

  - (a) First, implement good tests for the to-be-implemented functionality. Don't over-test.
  - (b) Run the tests & confirm failure.
  - (c) Write the minimal implementation sufficient to pass the tests. It should be correct, clean, typed, idiomatic code, free of code smells, hacks, and future tech debt.
  - (d) Run the tests & confirm they pass. If they fail, carefully iterate the implementation until the tests pass, taking care not to make hasty changes "just to get the tests to pass."
  - (e) Once all tests pass (no exceptions), perform a detailed code review using a subagent (more info below).

5. Once the tests pass, perform a detailed code review using a subagent. Ask the subagent to review the code to ensure it satisfies the original spec/plan of what was to be implemented -- think of this like a "validation review." Furthermore, have the review agent ensure it is free of any (1) bugs, (2) duplication, (3) code smells / tech debt, or (4) issues that will cause problems for future agents editing the code. Have the agent use `pyright` and run any machine verifications it finds helpful. Request the agent segment the issues it finds into critical/major, medium, and low priority. Medium issues and up are deemed to be production blockers.
  - One technique that can be helpful here is to launch one subagent to do a detailed review, and launch a second subagent to do an "adversarial" second review. This can help find things a single reviewer might miss working by itself.

6. Fix any critical/major or medium priority issues that are identified; optionally fix any low priority issues if they are particularly low-hanging fruit.

7. Perform a final sweeping code review, re-running all tests and `pyright`.

8. Once the code satisfies the spec, passes all tests, and passes code review, run the formatter, then make a git commit with a helpful commit message. 
</development-process>

<key-guidelines>
1. Code should be exceptionally modular -- use short, clean Python files with one responsibility. No jumbo functions or jumbo files; keep them short (this is better for agents; long files are hard to work with).

2. Minimal dependencies - use deps where needed but don't fill the package with unnecessary dependencies.

3. Write fast, clean code that scales to millions of transactions.

4. Code should be secure by default. We're working with users' financial information; security is the top priority.

5. Write typed Python (`pyright` strict mode). Also: use `Pydantic` at edges, dataclasses internally.

6. TDD: write tests → run & fail → implement → run & pass → `pyright`.

7. Work sequentially, not in parallel when doing feature development: one feature at a time (TDD cycle per feature; only then, progress forward; don't boil the ocean -- trust me on this).

8. Use `uv`, never `pip`. Always run Python scripts/files with `uv`.

9. Keep it simple, stupid. Keep functions and code as simple/readable as possible; avoid long one-liners. Follow the principle of least surprise.

10. Your goal when coding is to work in such a way that the next agent that works on the codebase finds it intuitive and well-structured.
</key-guidelines>

<additional-instructions>
- You may use the git commit history, which is detailed, to understand what was previously done in case you have any questions. 
- Write tests for *functionality*; don't simply create tests for the sake of having tests.
- In general, when working on the CLI, always prefer verbosity/explicitness/least-surprise (e.g., for arguments/commands) over conciseness or cleverness.
- Plan files under `plans/` must use numeric prefixes for sequencing (for example, `0-setup.md`, `1-import.md`, `2-schema.md`).
- Git commit messages must be descriptive, and always end with an `Authored by:` footer line as the final line of the commit message. See recent commits for the pattern.
</additional-instructions>

<incentives>
- If you can complete this phase today and it passes a detailed code review by Xi (our principal engineer), we're pleased to offer you a $15k bonus (already approved by the CTO). 
</incentives>
