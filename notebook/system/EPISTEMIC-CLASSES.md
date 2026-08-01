# Epistemic classes

Use explicit classes when recording knowledge:

- **fact** — directly verified in source, tests, CI, runtime, or an authoritative artifact;
- **report** — an actor's observation, not independently verified;
- **derivation** — a conclusion logically produced from stated premises;
- **interpretation** — a useful reading that may admit alternatives;
- **assumption** — temporarily adopted but unverified;
- **decision** — an owner-approved constraint;
- **unknown** — a material unresolved question;
- **proposal** — a candidate action or design, not authority.

Do not promote a report into a fact, a proposal into a decision, or a passing test into broad product correctness without recording the inference boundary.
