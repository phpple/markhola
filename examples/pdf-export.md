# PDF Export Verification

This file is used to verify the **v0.7.1 PDF export** flow.

## Checklist

- Export should include this heading and paragraph text
- Export should keep list indentation and numbering
- Export should keep code block formatting
- Export should keep image rendering

## Code Block

```rust
fn main() {
    println!("markhola pdf export");
}
```

## Table

| Item | Expectation |
| --- | --- |
| Preview | Renders normally |
| PDF | Matches the rendered content |

## Image

![MarkHola Diagram](./assets/diagram.svg)

## Math

Inline math: $E = mc^2$

$$
\int_0^1 x^2 \, dx = \frac{1}{3}
$$
