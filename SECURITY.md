# Security Policy

Rustit is experimental and has no supported production release yet. Security reports are still valuable, especially for dependency risk, parsing untrusted exchange files, database boundaries, vendor credentials, and future AI/agent authorization.

## Report privately

Use GitHub's **Security → Report a vulnerability** flow when private vulnerability reporting is available. Do not open a public issue for an unpatched vulnerability, leaked credential, or customer-data exposure.

If private reporting is unavailable, contact the bootstrap maintainer through the public contact method on [@jonathanmcmichael's GitHub profile](https://github.com/jonathanmcmichael) without including exploit details, then arrange a private channel.

## Include

- the affected commit, crate, or workflow;
- the smallest safe reproduction;
- expected impact and preconditions;
- whether the issue is already public; and
- suggested mitigation, if known.

Use synthetic data. Never attach customer models, schedules, credentials, tokens, or proprietary classification catalogs.

## Response

The maintainer will acknowledge a valid report as capacity permits, coordinate a fix and disclosure, and credit reporters who want attribution. No response-time guarantee exists during the bootstrap phase; that limitation will be revisited before any production-support claim.

## Supported versions

Only the current `main` branch receives security fixes until Rustit publishes versioned releases.
