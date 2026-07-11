# Programming Language Examples

This document is used to verify fenced code block rendering and syntax highlighting in MarkHola.

## Rust

```rust
fn main() {
    let languages = ["rust", "python", "javascript"];
    for language in languages {
        println!("highlighting: {language}");
    }
}
```

## Python

```python
def fibonacci(limit: int) -> list[int]:
    values = [0, 1]
    while values[-1] + values[-2] <= limit:
        values.append(values[-1] + values[-2])
    return values


print(fibonacci(55))
```

## JavaScript

```javascript
const formatUser = ({ id, name }) => `${id}: ${name.toUpperCase()}`;

console.log(formatUser({ id: 7, name: "markhola" }));
```

## TypeScript

```typescript
type Release = {
  version: string;
  features: string[];
};

const current: Release = {
  version: "0.6.3",
  features: ["highlighting", "mermaid", "finder-open"]
};
```

## Go

```go
package main

import "fmt"

func sum(values []int) int {
	total := 0
	for _, value := range values {
		total += value
	}
	return total
}

func main() {
	fmt.Println(sum([]int{3, 5, 8}))
}
```

## Java

```java
import java.util.List;

public class Example {
    public static void main(String[] args) {
        List<String> names = List.of("Ada", "Linus", "Grace");
        names.stream().map(String::toLowerCase).forEach(System.out::println);
    }
}
```

## Swift

```swift
struct Article {
    let title: String
    let tags: [String]
}

let article = Article(title: "MarkHola", tags: ["markdown", "swift", "macOS"])
print(article.tags.joined(separator: ", "))
```

## Kotlin

```kotlin
data class Ticket(val issue: String, val resolved: Boolean)

val tickets = listOf(
    Ticket("open menu", true),
    Ticket("finder open", true)
)

println(tickets.count { it.resolved })
```

## C

```c
#include <stdio.h>

int main(void) {
    const char *message = "MarkHola";
    printf("%s supports Markdown previews.\n", message);
    return 0;
}
```

## C++

```cpp
#include <iostream>
#include <vector>

int main() {
    std::vector<int> values {1, 4, 9, 16};
    for (const auto value : values) {
        std::cout << value << std::endl;
    }
}
```

## Shell

```bash
#!/usr/bin/env bash
set -euo pipefail

for file in examples/*.md; do
  echo "preview sample: ${file}"
done
```

## JSON

```json
{
  "name": "markhola",
  "version": "0.6.3",
  "features": ["readonly", "writable", "mermaid", "finder-open"]
}
```

## YAML

```yaml
app:
  name: MarkHola
  platform: macOS
  features:
    - code-highlight
    - mermaid
    - open-from-finder
```

## SQL

```sql
SELECT version, feature_count
FROM releases
WHERE version >= '0.6.1'
ORDER BY version DESC;
```

## HTML

```html
<article class="note">
  <h1>MarkHola</h1>
  <p>Preview local Markdown with syntax highlighting.</p>
</article>
```

## CSS

```css
.code-sample {
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 12px;
  padding: 16px;
  background: #fffdf8;
}
```

## XML

```xml
<release version="0.6.3">
  <feature>Mermaid</feature>
  <feature>FinderOpen</feature>
</release>
```

## Plain Text Fallback

```text
If a language is unsupported, the code block should still render safely.
```
