// Batch tier for the `design-implement` skill.
//
// Runs the skill's pipeline across several design surfaces in one pass — one
// isolated worktree, branch, and PR per surface, in parallel. Opt-in only: this
// spawns one agent per surface and each opens a PR.
//
// Run it with the Workflow tool, passing the surfaces as `args`, e.g.
//   args: ["Approvals", "Projects", "Reports", "Settings"]
// Each string must match a `design/project/app/Horae <Surface>.dc.html` file.

export const meta = {
  name: 'design-implement-batch',
  description: 'Implement several design surfaces from design/ into one PR each',
  phases: [{ title: 'Implement' }],
}

const surfaces = Array.isArray(args) && args.length ? args : ['Approvals', 'Projects', 'Reports', 'Settings']
log(`Implementing ${surfaces.length} design surface(s): ${surfaces.join(', ')}`)

const RESULT = {
  type: 'object',
  properties: {
    surface: { type: 'string' },
    branch: { type: 'string' },
    prUrl: { type: 'string' },
    status: { type: 'string', enum: ['pr-opened', 'blocked'] },
    deviations: { type: 'string', description: 'Scope deviations from the mockup, or empty' },
    notes: { type: 'string' },
  },
  required: ['surface', 'status', 'notes'],
}

// One surface per agent, each in its own git worktree so parallel file edits
// never collide. Each agent owns the full pipeline: read the .dc.html, implement,
// verify, open the PR.
const results = await parallel(
  surfaces.map((surface) => () =>
    agent(
      [
        `Implement the "${surface}" design surface for the Horae app.`,
        '',
        'REQUIRED: follow the `design-implement` skill end to end — read',
        `\`design/project/app/Horae ${surface}.dc.html\` in full and its imports,`,
        'style ONLY with the Tailwind-esque utility classes + tokens from',
        'assets/css/horae.css (no inline styles, no hardcoded hex), reuse existing',
        'components, stay data-honest, and verify with',
        '`SQLX_OFFLINE=true cargo clippy -p horae --features server -- -D warnings`,',
        '`cargo check -p horae --features web --target wasm32-unknown-unknown`, and',
        '`nix fmt` before finishing.',
        '',
        'If the surface has no src/pages counterpart, scaffold a new screen (page',
        'module in src/pages.rs, a #[route] in src/route.rs, a sidebar nav entry).',
        '',
        'Open ONE PR with a plain, human-style title and body (NO AI attribution,',
        'no Co-Authored-By). List any scope deviations from the mockup in the body.',
        'Do not merge.',
      ].join('\n'),
      { label: `design:${surface}`, phase: 'Implement', isolation: 'worktree', schema: RESULT },
    ),
  ),
)

const done = results.filter(Boolean)
log(`Opened ${done.filter((r) => r.status === 'pr-opened').length}/${surfaces.length} PR(s)`)
return done
