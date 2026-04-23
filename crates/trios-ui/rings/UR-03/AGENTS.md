# AGENTS.md — UR-03

## Agent: ALPHA
- Add resizable panel (drag to resize)
- Add split panel (horizontal/vertical split)
- Add breadcrumb navigation component

## Agent: BETA
- Test sidebar collapse/expand animation
- Test tab switching with keyboard
- Verify responsive layout in narrow sidebar

## Agent: DELTA
- Fix compilation (blocked on UR-00 + UR-01)
- Remove unused `trios-ui-ur02` from Cargo.toml if not needed

## Rules
- R1: Layout components read state from UR-00 atoms only
- R2: All dimensions use UR-01 spacing tokens
- R3: Helper functions (`render_nav_item`, `render_tab`) return `Element`
