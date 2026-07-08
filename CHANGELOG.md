# Changelog

## [0.2.0] - 2026-07-07

### Changed
- `Rounding::Round` now returns the expected integer for values just below a half step, so `0.49999999999999994` rounds to `0`. (#20)
- Single-element arrays that contain booleans or dates now fail numeric serialization instead of producing a number. (#21)

### Performance
- Branch selection now reuses compiled `pattern` regular expressions, which reduces repeated validation work for schemas that use `pattern`. (#19)
- Branch selection with `additionalProperties: false` now checks declared property names directly, which reduces work for objects with extra keys. (#22)

## [0.2.0] - 2026-07-07

### Changed
- `Rounding::Round` now returns the expected integer for values just below a half step, so `0.49999999999999994` rounds to `0`. (#20)
- Single-element arrays that contain booleans or dates now fail numeric serialization instead of producing a number. (#21)

### Performance
- Branch selection now reuses compiled `pattern` regular expressions, which reduces repeated validation work for schemas that use `pattern`. (#19)
- Branch selection with `additionalProperties: false` now checks declared property names directly, which reduces work for objects with extra keys. (#22)
