# template-filling Library

选择语言 / Select Language: [English](/README.md) | [简体中文](/docs/README.zh-CN.md)

## Project Goals

- **High Performance**: The template filling library is optimized to handle complex template logic efficiently, ensuring high performance.
- **Low Memory Usage**: Designed to be lightweight, making it suitable for embedded environments and resource-constrained scenarios.

## Special Notes

- **Potential API Changes**: The project is still under development, and the API may undergo significant changes. Please keep an eye on the changelog.
- **Gradual Feature Enhancement**: Advanced features are being developed and improved gradually. Stay tuned for updates.

## Features

- **Template Filling**: Supports complex template logic, including conditional statements, loops, and variable substitution.
- **Unicode Escape**: Provides functionality for escaping and decoding Unicode characters.
- **Flexible Environment Variable Support**: Allows dynamic setting and usage of environment variables.
- **Multi-line Definition Support**: Enables multi-line definitions and nested templates.

## Usage Examples

### Example 1: Basic Template Filling

```rust
use template_filling::fill;

fn main() {
  let template = "Hello, {{name}}!";
  let data = serde_json::json!({ "name": "World" });
  let result = fill(template, Some(&data));
  println!("{}", result); // Output: Hello, World!
}
```

### Example 2: Conditional Statements and Loops

```rust
use template_filling::fill;

fn main() {
  let template = r#"
  {% if is_active %}
    Active User: {{name}}
  {% endif %}
  {% for item in items %}
    Item: {{item}}
  {% endfor %}
  "#;
  let data = serde_json::json!({
    "is_active": true,
    "name": "Gin",
    "items": ["Item1", "Item2", "Item3"]
  });
  let result = fill(template, Some(&data));
  println!("{}", result);
}
```

### Example 3: CLI Command Usage

```bash
# Fill a single template
template_filling_cli fill -p ./template.tmpl -d '{"name":"World"}' -o ./output.txt

# Batch fill templates
template_filling_cli batch_fill -p ./templates -t demo -d '{"name":"World"}' -o ./outputs
```

## License

This project is open-sourced under the [MIT License](https://opensource.org/licenses/MIT). Contributions and suggestions are welcome. template-filling
