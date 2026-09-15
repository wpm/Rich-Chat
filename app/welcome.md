# Welcome to Rich Chat

Everything you type in the box below renders **as you type**: Markdown, LaTeX math, and code in some two hundred languages. This bubble is a tour.

## Markdown

CommonMark plus GitHub's extras: *emphasis*, **strong**, ~~struck~~, `inline code`, [links](https://github.com/wpm/Rich-Chat), and footnotes[^1].

| Construct | Syntax | Rendered by |
|---|:--:|---|
| Tables | `\|` | pulldown-cmark |
| Math | `$…$` | pulldown-latex |
| Code | ` ``` ` | syntect |

- [x] task lists
- [x] tables
- [ ] whatever you send next

> [!TIP]
> Enter sends. Shift+Enter starts a new line, so a code fence is easy to write.

## Code

Fenced blocks are highlighted and get a copy button. Rust:

```rust
/// Newton's method, with the derivative closed over.
fn newton(f: impl Fn(f64) -> f64, df: impl Fn(f64) -> f64, mut x: f64) -> f64 {
    for _ in 0..50 {
        let step = f(x) / df(x);
        x -= step;
        if step.abs() < 1e-12 {
            break;
        }
    }
    x
}

fn main() {
    let root = newton(|x| x * x - 2.0, |x| 2.0 * x, 1.0);
    println!("√2 ≈ {root:.12}");
}
```

Python:

```python
from functools import lru_cache

@lru_cache(maxsize=None)
def partitions(n: int, k: int | None = None) -> int:
    """Ways to write n as a sum of positive integers no larger than k."""
    k = n if k is None else k
    if n == 0:
        return 1
    if n < 0 or k == 0:
        return 0
    return partitions(n - k, k) + partitions(n, k - 1)

print([partitions(n) for n in range(10)])  # 1, 1, 2, 3, 5, 7, 11, 15, 22, 30
```

## Math

Inline math sits in a sentence, like $e^{i\pi} + 1 = 0$ or $\binom{n}{k} = \frac{n!}{k!\,(n-k)!}$. Display math gets its own line, set in Latin Modern, the TeX font, as native MathML with no JavaScript:

$$
\oint_{\partial\Omega} \mathbf{F} \cdot d\mathbf{S}
  = \iiint_{\Omega} \left( \nabla \cdot \mathbf{F} \right) dV
$$

Maxwell's equations, aligned:

$$
\begin{aligned}
  \nabla \cdot \mathbf{E} &= \frac{\rho}{\varepsilon_0} &
  \nabla \times \mathbf{E} &= -\frac{\partial \mathbf{B}}{\partial t} \\[4pt]
  \nabla \cdot \mathbf{B} &= 0 &
  \nabla \times \mathbf{B} &= \mu_0 \mathbf{J} + \mu_0 \varepsilon_0 \frac{\partial \mathbf{E}}{\partial t}
\end{aligned}
$$

And a piecewise definition with a matrix:

$$
f(x) =
\begin{cases}
  \displaystyle \sum_{n=0}^{\infty} \frac{(-1)^n}{(2n+1)!}\, x^{2n+1} & \text{if } |x| < \pi \\[6pt]
  \det \begin{pmatrix} \cos x & -\sin x \\ \sin x & \cos x \end{pmatrix} & \text{otherwise}
\end{cases}
$$

[^1]: Footnotes collect at the end of the message, GitHub style.
