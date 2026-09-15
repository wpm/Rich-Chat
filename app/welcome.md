# Welcome to Rich Chat

Everything you type in the box below renders **as you type**: Markdown, LaTeX math, and code in some two hundred languages. These two bubbles are a tour.

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

Fenced blocks are highlighted and get a copy button:

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


[^1]: Footnotes collect at the end of the message, GitHub style.
