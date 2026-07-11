# Mathematical Expression Examples

This document is used to verify GitHub-style mathematical expression rendering in MarkHola.

## Inline Math With Dollar Delimiters

Euler's identity is $e^{i\pi} + 1 = 0$.

The quadratic roots are $x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$.

## Inline Math With Backtick-Aware Delimiters

This sentence uses $`\sqrt{3x-1} + (1+x)^2`$ to avoid Markdown conflicts.

## Display Math With Double Dollars

$$
\left( \sum_{k=1}^n a_k b_k \right)^2
\le
\left( \sum_{k=1}^n a_k^2 \right)
\left( \sum_{k=1}^n b_k^2 \right)
$$

$$
\int_0^1 x^2 \, dx = \frac{1}{3}
$$

## Fenced Math Block

```math
\nabla \cdot \vec{E} = \frac{\rho}{\varepsilon_0}
```

## Dollar Sign Examples

Inside math use an escaped dollar sign: $`\sqrt{\$4}`$.

Outside math but on the same line use HTML span tags: To split <span>$</span>100 in half, we calculate $100/2$.
