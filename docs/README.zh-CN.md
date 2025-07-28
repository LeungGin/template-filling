# template-filling库

## 项目目标

- **性能优越**: 模板填充库经过优化，能够快速处理复杂的模板逻辑，确保高效运行。
- **低内存占用**: 设计轻量化，适合嵌入式环境和资源受限的场景。

## 特别说明

- **API可能大面积变更**: 项目仍在开发中，API可能会发生较大调整，请关注更新日志。
- **高级功能逐步完善**: 高级功能正在逐步开发和完善，敬请期待。

## 功能介绍

- **模板填充**: 支持复杂的模板逻辑，包括条件判断、循环、变量替换等。
- **Unicode转义**: 提供对Unicode字符的转义和解码功能。
- **灵活的环境变量支持**: 支持动态设置和使用环境变量。
- **多行定义支持**: 支持多行定义和嵌套模板。

## 使用示例

### 示例 1: 基本模板填充

```rust
use template_filling::fill;

fn main() {
  let template = "你好, {{name}}!";
  let data = serde_json::json!({ "name": "世界" });
  let result = fill(template, Some(&data));
  println!("{}", result); // 输出: 你好, 世界!
}
```

### 示例 2: 条件判断和循环

```rust
use template_filling::fill;

fn main() {
  let template = r#"
  {% if is_active %}
    活跃用户: {{name}}
  {% endif %}
  {% for item in items %}
    项目: {{item}}
  {% endfor %}
  "#;
  let data = serde_json::json!({
    "is_active": true,
    "name": "Gin",
    "items": ["项目1", "项目2", "项目3"]
  });
  let result = fill(template, Some(&data));
  println!("{}", result);
}
```

### 示例 3: CLI命令行使用

```bash
# 填充单个模板
template_filling_cli fill -p ./template.tmpl -d '{"name":"世界"}' -o ./output.txt

# 批量填充模板
template_filling_cli batch_fill -p ./templates -t demo -d '{"name":"世界"}' -o ./outputs
```

## 开源协议

本项目基于 [MIT License](https://opensource.org/licenses/MIT) 开源，欢迎贡献代码和提出建议。
