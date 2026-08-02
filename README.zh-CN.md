<h1 align="center">LON</h1>

<p align="center">
  <a href="spec/lon-v1.md">
    <img src="assets/lon-logo.svg" alt="LON——紧凑、无损、AI-first 的对象表示法" width="640">
  </a>
</p>

<p align="center">
  <a href="https://pub.dev/packages/lon"><img alt="包版本 1.0.0" src="https://img.shields.io/badge/version-1.0.0-8B5CF6?style=flat-square"></a>
  <a href="spec/lon-v1.md"><img alt="LON 格式 v1" src="https://img.shields.io/badge/format-LON_v1-22D3EE?style=flat-square"></a>
  <a href="pubspec.yaml"><img alt="Dart SDK 3.3 或更高版本" src="https://img.shields.io/badge/Dart-%5E3.3.0-0175C2?style=flat-square&amp;logo=dart&amp;logoColor=white"></a>
  <a href="spec/lon-v1.md"><img alt="无损 JSON" src="https://img.shields.io/badge/JSON-lossless-2EA44F?style=flat-square&amp;logo=json&amp;logoColor=white"></a>
  <a href="benchmark/results/token-efficiency.md"><img alt="o200k benchmark 以 LON 为基准" src="https://img.shields.io/badge/o200k-LON_baseline-F59E0B?style=flat-square"></a>
  <a href="https://github.com/fluttercandies/lon"><img alt="GitHub 仓库" src="https://img.shields.io/badge/source-GitHub-181717?style=flat-square&amp;logo=github&amp;logoColor=white"></a>
</p>

<p align="center">
  <a href="README.md">English</a> · <strong>简体中文</strong>
</p>

Language Object Notation（LON，语言对象表示法）是面向 JSON 数据模型的紧凑、规范化、无损文本编码。它以 AI-first 数据交换为目标，同时不让人类或模型猜测类型、边界、schema 或被省略的结构。

> 包版本：`1.0.0` · 格式版本：`LON v1` · Dart SDK：`^3.3.0`

## 为什么选择 LON

- **JSON 文本无损转换。** 保留对象成员顺序、重复成员、字符串代码单元和数字原始词法。
- **AI-first，但不只服务 AI。** 删除冗余语法，同时保留熟悉的 JSON 定界符和明确的类型边界。
- **自描述表格。** 重复记录键可提取为可见 schema，后续各行使用分号分隔。
- **唯一规范输出。** 解码器接受的不同等价写法会规范化为同一个确定性输出。
- **没有隐藏上下文。** 不使用别名、键字典、数字引用、外部 schema 或模型指令。
- **资源有界。** 在不安全分配或乘法发生前限制解析、展开、嵌套、输入和输出。

## 快速示例

JSON：

```json
{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"},{"id":3,"name":"Lin"}]}
```

规范 LON：

```lon
{users:[id name;1 Ada;2 Bob;3 Lin]}
```

载荷本身仍然自描述：

- `[` 和 `]` 仍是解码后数组的边界。
- `id name` 是可见记录 schema；第一个 `;` 结束 schema。
- 后续每个 `;` 开始新行，行宽必须与 schema 完全一致。
- 空格分隔字段，不再增加行圆括号或另一种分隔符。

对于小型或非统一数据，LON 保持接近 JSON：

```json
{"id":7,"name":"Ada","active":true}
```

```lon
{id:7 name:Ada active:true}
```

## 一分钟了解语法

### 对象和数组

对象保留花括号和冒号，以空白替代对象成员之间的逗号：

```lon
{name:Ada age:37 active:true}
```

数组保留方括号。只有全部由安全裸字符串组成的数组才使用空格：

```lon
[Ada Bob Lin]
```

其他数组保留逗号，使混合类型和嵌套边界保持明确：

```lon
[1,Ada,true,null]
[[1,2],[3,4]]
[1,{a:b},{a:[1,{b:c}]}]
```

### 确定性的值类型

值分类只有一个完备的优先顺序：

1. 精确的 `null`、`true` 和 `false` token 是 JSON literal。
2. 符合 JSON number 语法的 token 是数字。
3. 其他允许的裸 token 全部是字符串。
4. 不安全或有歧义的内容必须使用 JSON 双引号字符串。

LON 不推断 identifier、enum、symbol、date 或自定义标量类型。例如，下面两个值都是字符串：

```lon
[Ada 2026-08-02]
```

这些易误判字符串保持引用：

```lon
["01" ".5" "NaN" "undefined" "None" "yes" "off"]
```

包含空白、结构标点、双引号、反斜杠、控制字符、不可见分隔符或孤立 surrogate 的字符串也必须使用 JSON 引用或转义。LON 不对 Unicode 的大小写、宽度、方向、NFC 或 NFD 做归一化。
相同规则也适用于对象成员名和表格 schema 字段名，包括空 key 及任意 JSON 字符串代码单元序列。

### 自描述表格

统一记录数组可以使用：

```text
[SCHEMA;ROW;ROW]
```

带嵌套 schema 的示例：

```lon
[id name addr{city zip} active;1 Ada London W1 true;2 Bob Paris "75001" false]
```

等价 JSON：

```json
[
  {"id":1,"name":"Ada","addr":{"city":"London","zip":"W1"},"active":true},
  {"id":2,"name":"Bob","addr":{"city":"Paris","zip":"75001"},"active":false}
]
```

Schema 和表格分别至少需要两个叶子值字段及两行。更小的记录保持普通数组形式，因为 `[id;1;2]` 和 `[id name;1 Ada]` 在没有格式说明时很容易被误认为标量数组或对象。

键控对象和含有可变 object cell 形状的数组保持显式形式：

```lon
{production:{region:eu-central-1 replicas:6 debug:false} staging:{region:eu-central-1 replicas:2 debug:true}}
```

外层方括号始终解码为数组。合法 schema 后的 `;` 会唯一选择表格语法，因为普通数组绝不使用分号作为分隔符。空格分隔 cell，后续分号分隔行，因此不需要数量、行圆括号或重复分隔符。没有声明行数时无法检测完整行被整体删除；需要防遗漏时应由传输层提供校验和或认证信封。

完整语法和规范表格选择规则见 [LON v1 规范](spec/lon-v1.md)。

## 在 Dart 中使用

从 pub.dev 安装包：

```sh
dart pub add lon
```

也可以直接添加依赖：

```yaml
dependencies:
  lon: ^1.0.0
```

本地开发时，将 hosted 版本约束替换为 `path: /path/to/lon`。

导入公开 API：

```dart
import 'package:lon/lon.dart';
```

### 无损文本到文本 API

需要保留重复对象成员和精确数字词法时，使用文本 API：

```dart
const json = '{"value":1e+02,"value":-0,"name":"Ada"}';

final encoded = jsonToLon(json);
final decodedJson = lonToJson(encoded);

print(encoded);     // {value:1e+02 value:-0 name:Ada}
print(decodedJson); // {"value":1e+02,"value":-0,"name":"Ada"}
```

`jsonToLon` 接受严格 JSON 文本并输出规范 LON。`lonToJson` 接受 LON 并输出 compact JSON，同时保留成员顺序、重复成员和数字词法。

### Dart 值 API

普通 JSON-compatible Dart 值可使用默认 codec：

```dart
final encoded = lon.encode({
  'name': 'Ada',
  'roles': ['admin', 'editor'],
  'active': true,
});

final value = lon.decode(encoded);
```

Dart 值 API 遵循普通 Dart/JSON map 行为：无法表达重复键，解码重复成员时保留最后一个值。需要保留这些差异时，应使用文本到文本 API。

### 自定义限制

```dart
const codec = LonCodec(
  useTables: true,
  maxDepth: 128,
  maxExpandedElements: 100000,
  maxInputCodeUnits: 8 * 1024 * 1024,
  maxOutputCodeUnits: 8 * 1024 * 1024,
);
```

默认限制：

| 限制 | 默认值 |
|---|---:|
| 容器嵌套深度 | 512 |
| 已解析或展开元素数 | 1,000,000 |
| table schema 字段数 | 1,000,000，通过元素限制约束 |
| 输入 UTF-16 代码单元 | 67,108,864 |
| 输出 UTF-16 代码单元 | 67,108,864 |

文本解析和限制失败抛出 `FormatException`。无效的负数 codec 限制抛出 `RangeError`。Dart 值编码遇到不支持、循环、非有限数值或超预算值时抛出参数错误，不返回部分文档。

## 命令行

在当前仓库中运行：

```sh
dart pub get
dart run bin/lon.dart encode input.json
dart run bin/lon.dart decode input.lon
```

未提供文件时从标准输入读取：

```sh
printf '%s' '{"name":"Ada","active":true}' | dart run bin/lon.dart encode
printf '%s' '{name:Ada active:true}' | dart run bin/lon.dart decode
```

从 pub.dev 安装可执行文件：

```sh
dart pub global activate lon
lon encode input.json
lon decode input.lon
```

如需激活本地 checkout，运行 `dart pub global activate --source path .`。

CLI 行为：

| 退出码 | 含义 |
|---:|---|
| 64 | 命令或参数数量无效 |
| 65 | JSON/LON 无效、UTF-8 无效或超过输入限制 |
| 66 | 文件系统失败 |

CLI 将流式 UTF-8 输入限制为 67,108,864 字节，并且不会在诊断中回显不受信任的输入。

### UTF-8 与网络传输

将一份完整 LON 文档编码为不带 BOM 的严格 UTF-8；接收端必须拒绝非法 UTF-8，不能用替换字符容错解码：

```dart
import 'dart:convert';

final body = utf8.encode(lon.encode(value));
final received = lon.decode(utf8.decode(body, allowMalformed: false));
```

通过 HTTP 或其他保字节协议传输时，应把 LON 放在消息 body 中，并提供明确的内容长度或 frame 边界。当前包没有注册媒体类型；要求字节完全一致时使用 `application/octet-stream`，只有在中间层不会归一化或转码时才使用 `text/plain; charset=utf-8`。

规范 LON 不会从字符串数据中输出原始 C0/C1 控制字符、CR、LF、Unicode 行分隔符、BOM 或影响可见性的码点；它们都会成为可见 JSON 转义。CLI 只会在完整文档末尾额外写入一个 LF。LON 不是 HTTP header、URL、HTML、JavaScript、shell 或 SQL 的转义格式，不能把不受信任的 LON 原样嵌入这些上下文，必须继续使用目标上下文自己的编码规则。

## Token benchmark

可复现 benchmark 在 27 个确定性小型、中型和大型场景中比较 LON、Compact JSON、TOON 4.1.0、YAML 2.9.0 和 Pretty JSON。每份载荷都必须先解码还原为原始 JSON 值，之后才会计数。

`o200k_base`，以 LON 为对比基准：

| 范围 | LON | Compact JSON | TOON | YAML | Pretty JSON |
|---|---:|---:|---:|---:|---:|
| 小型，14 个场景 | **149** | 193（+29.5%） | 186（+24.8%） | 202（+35.6%） | 347（+132.9%） |
| 中型，8 个场景 | **3,964** | 5,516（+39.2%） | 5,119（+29.1%） | 7,526（+89.9%） | 9,146（+130.7%） |
| 大型，5 个场景 | **79,694** | 107,636（+35.1%） | 104,247（+30.8%） | 134,543（+68.8%） | 168,891（+111.9%） |
| 全部，27 个场景 | **83,807** | 113,345（+35.2%） | 109,552（+30.7%） | 142,271（+69.8%） | 178,384（+112.9%） |

正百分比表示该格式比 LON 消耗更多 token。benchmark 不会强行让 LON 在所有极小载荷中胜出：报告完整保留平局以及 TOON 或 YAML 更小的场景。

复现 benchmark：

```sh
cd benchmark
bun install --frozen-lockfile
bun run benchmark
```

输出：

- [人类可读 benchmark 报告](benchmark/results/token-efficiency.md)
- [机器可读 benchmark 结果](benchmark/results/token-efficiency.json)

Token 数不是模型理解能力评分。此 benchmark 测量载荷成本；语义 round-trip 验证和序列化原文示例提供客观的正确性与可检查性依据。

## 正确性与一致性

对于有效 JSON 文本 `J`：

```text
lonToJson(jsonToLon(J))
```

生成具有相同 JSON 值、成员顺序、重复成员、字符串代码单元和数字词法的 compact JSON。JSON 空白和等价转义写法会被规范化。

对于规范 LON `L`：

```text
jsonToLon(lonToJson(L)) == L
```

运行质量门禁：

```sh
dart analyze
dart test
```

测试套件包括：

- [LON v1 一致性用例](conformance/v1.json)
- 确定性生成 round-trip
- JSONTestSuite 有效、无效和 implementation-defined 输入
- Unicode 和 surrogate round-trip
- 畸形与截断 table 拒绝
- 嵌套、schema、展开、输入和输出限制
- CLI 诊断安全

## 项目结构

| 路径 | 用途 |
|---|---|
| `lib/` | Dart 公开包与 codec 实现 |
| `bin/lon.dart` | `lon encode` / `lon decode` CLI |
| `assets/` | 可缩放 LON 横向 Logo 与独立标识 |
| `example/` | 可运行的包使用示例 |
| `spec/lon-v1.md` | 规范格式文档 |
| `conformance/v1.json` | 跨实现一致性 fixtures |
| `benchmark/` | 可复现多格式 token benchmark |
| `.github/workflows/benchmark.yml` | 每次推送运行 benchmark 的 CI |
| `test/` | 单元、round-trip、安全和 JSON 一致性测试 |
| `CHANGELOG.md` | 按版本记录的发布变更 |
| `LICENSE` | BSD 3-Clause 许可证 |

## 设计边界

LON 编码 JSON 数据模型。它不添加注释、时间戳、二进制 blob、引用、任意对象类型或推断式应用 schema。保持这条边界，才能强制保证规范 round-trip 以及 AI 和人类无歧义理解。

## 许可证

LON 使用 [BSD 3-Clause License](LICENSE)。
