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
