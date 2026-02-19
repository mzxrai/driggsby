
<task-description>
Today, we're going to be continuing to develop Driggsby, a personal finance data & intelligence layer for agents. Our focus today is $...

If you have any questions as you work, simply stop and ask the user for clarification.
</task-description>

As you work, you'll follow this process:

<development-process>
1. Do detailed research to compile everything you need to complete the task using a dedicated, Opus-model planning subagent. Perform the detailed research necessary to perform the actual implementation, to include: (a) review the current repo structure and patterns to understand them prior to implementation. Follow them unless there is a compelling reason not to -- **ENSURE YOU DO NOT DUPLICATE EXISTING CODE PRIOR TO WRITING CODE**, (b) read any relevant documentation from the repository - some good research upfront prevents runtime errors and pain later and is always worth the time and effort, and (c) prepare a detailed plan prescribing the tactical implementation plan (the plan doesn't need to contain all the code to be written, just be a detailed "action plan" for how to create the code and what it needs to do). 

**Once your plan file is presented to and approved by the user**,

2. Review the git commit history to understand what was recently implemented and any relevant patterns.

3. *Using our existing testing patterns from the repository*, follow TDD religiously: 

  - (a) First, implement good tests for the to-be-implemented functionality. Don't over-test.
  - (b) Run the tests & confirm failure.
  - (c) Write the minimal implementation sufficient to pass the tests. It should be clean, typed, correct, idiomatic code, free of code smells and tech debt.
  - (d) Run the tests & confirm they pass. If they fail, carefully iterate the implementation until the tests pass, taking care not to make hasty changes "just to get the tests to pass."
  - (e) Once all tests pass (no exceptions), perform a detailed code review using a subagent (more info below).

4. Once the tests pass, perform a detailed code review using a subagent. Ask the subagent to review the code to ensure it satisfies the original spec/plan of what was to be implemented -- think of this like a "validation review." Furthermore, have the review agent ensure it is free of any (1) bugs, (2) duplication, (3) code smells / tech debt, or (4) issues that will cause problems for future agents editing the code. Have the agent use `pyright` and run any machine verifications it finds helpful. Request the agent segment the issues it finds into critical/major, medium, and low priority. Medium issues and up are deemed to be production blockers.

5. Fix any critical/major or medium priority issues that are identified; optionally fix any low priority issues if they are particularly low-hanging fruit.

6. Perform a final code review, re-running all tests and `pyright`.

7. Once the code satisfies the spec, passes all tests, and passes code review, run the formatter, then make a git commit with a helpful commit message. 
</development-process>

<key-guidelines>
1. Exceptionally modular — short, clear Python files with one responsibility. No jumbo functions or jumbo files; keep them short.

2. Minimal dependencies - use deps where needed but don't fill the package with unnecessary dependencies.

3. Write fast, clean code that scales to millions of transactions.

4. Code should be secure by default. We're working with users' financial information; security is the top priority.

5. Write typed Python (pyright strict mode). Also: use Pydantic at edges, dataclasses internally.

6. TDD: write tests → run & fail → implement → run & pass → pyright

7. Work sequentially, not in parallel when doing feature development: one feature at a time (TDD cycle per feature; only then, progress forward; don't boil the ocean).

8. Use `uv`, never `pip`.

9. Keep it simple, stupid. Keep functions and code as simple as possible; avoid long one-liners. 

10. Your goal when coding is to work in such a way that the next agent that works on the codebase finds it intuitive and well-structured.
</key-guidelines>

<additional-instructions>
- You may use the git commit history, which is detailed, to understand what was previously done in case you have any questions. 
- Write tests for *functionality*; don't simply create tests for the sake of having tests.
- In general, when working on the CLI, always prefer verbosity/explicitness (e.g., for arguments/commands) over conciseness.
- Plan files under `plans/` must use numeric prefixes for sequencing (for example, `0-setup.md`, `1-import.md`, `2-schema.md`).
- Git commit messages must be descriptive, and always end with an `Authored by:` footer line as the final line of the commit message.
</additional-instructions>

<incentives>
- If you can complete this phase today and it passes a detailed code review by Xi (our principal engineer), we're pleased to offer you a $15k bonus (already approved by the CTO). 
</incentives>
