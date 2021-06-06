# Relation

用于定义两个 `Meta` 的上下游关系，其定义存储到数据表 relation 中。

## 定义 `Relation`

示例如下：

```sql
INSERT INTO relation
(from_meta, to_meta, settings)
VALUES('B:sale/order:1', 'B:sale/orderState:1', '{"target_states":{"add":["new"]}}');
```

## 定义 `Relation` 的处理方式

即使两个 `Meta` 建立了 `Relation` 也不一定可以执行，还要看 `settings` 里的设置，`settings` 为 JSON字符串，其内容定义如下：

```json
{
    "selector": {...},		// 缺省为 null, 选择符合条件的上游。见下面的“selector”
    "down_selector": {...},// 缺省为 null, 选择符合条件的下游。见下面的“down_selector”
    "executor": {...},		// 缺省为 null, 指定 Executor。见下面的 “Executor”
    "convert_before": [{...}],	// 前置 Executor, 可以指定多个，按给定的顺序执行。
    "convert_after": [{...}],	// 后置 Executor, 可以指定多个，按给定的顺序执行。
    "use_upstream_id": bool,	// 新生成的 Instance.id 将使用上游 Instance.id
    "target": {},			// 缺省为 null, 对下游 Instance 进行干预下游，见下面的“干预下游”
    "delay": 0,				// 缺省为 0，从当前时间延迟指定的秒数后执行本任务
    "delay_on_para": [100,2],	// 缺省为 null，延迟执行。数组中的第一个值为延迟的秒数，第二个值为基础时间的位置，该位置位于上游 Instance.para 中。
    "id_bridge": bool,		// 缺省为 false, 下游不使用上游的id，但下游的下游会用到，则需要将此值设为true
}
```

### selector

上游必须满足指定条件 Nature 才可以调用 `Executor`。这些条件的定义如下：

```json
{
    "state_all": ["s1"],	// 缺省为 null, 上游必须满足全部指定的状态
    "state_any": ["s1"],	// 缺省为 null, 上游需要满足其中的一个状态
    "state_none": ["s1"],	// 缺省为 null, 上游不能包含任何给定的状态
    "context_all": ["c1"],	// 缺省为 null, 上游必须满足全部指定的 context
    "context_any": ["c1"],	// 缺省为 null, 上游需要满足其中的一个 context
    "context_none": ["c1"],	// 缺省为 null, 上游不能包含任何给定的 context
    "sys_context_all": ["c1"],	// 缺省为 null, 上游必须满足全部指定的 sys_context
    "sys_context_any": ["c1"],	// 缺省为 null, 上游需要满足其中的一个 sys_context
    "sys_context_none": ["c1"],	// 缺省为 null, 上游不能包含任何给定的 sys_context
}
```

条件的检查顺序为：xxx_none，xxx_all，xxx_any。

**注意**：尽管`context`和`sys_context`都是 KV 类型，但当做流程选择条件时，Nature 只处理“K”不处理“V”，这是从简化设计角度来考虑的。“V”的形式是业务决定的，可能是一个URL，“a|b|c”，或者是个json，所以是不规范的。Nature 也不想对此进行规范，这样可能既限制了业务的灵活性又降低了处理性能。而“K”则是非常规范的，就是一个标签，非常便于 Nature 进行处理。当然这种方式也有问题，当`context`和`sys_context`用作流程选择时就失去了KV的意义。如根据性别选择不同的处理流程：

- 错误的方式：

  | KEY    | VALUE           |
  | ------ | --------------- |
  | gender | "boy" \| "girl" |

- 正确方式1：

  | KEY                       | VALUE |
  | ------------------------- | ----- |
  | gender.boy \| gender.girl | ""    |

  流程控制设置类似于：

  - 男孩流程：relation1.selector.**context_all** = ["gender.boy"]

  - 女孩流程：relation2.selector.**context_all** = ["gender.girl"]

- 正确方式2：

  | KEY          | VALUE |
  | ------------ | ----- |
  | gender.isBoy | ""    |
  
  流程控制设置类似于：
  
  - 男孩流程：relation1.selector.**context_all** = ["gender.isBoy"]
  
  - 女孩流程：relation2.selector.**context_none** = ["gender.isBoy"]

### down_selector

下游必须满足指定条件 Nature 才可以调用 `Executor`。这些条件的定义如下：

```json
{
    "last_all": ["s1"],		// 缺省为 null, 下游上一版本必须满足全部指定的状态
    "last_any": ["s1"],		// 缺省为 null, 下游上一版本需要满足其中的一个状态
    "last_none": ["s1"],	// 缺省为 null, 下游上一版本不能包含任何给定的状态
}
```

条件的检查顺序为：xxx_none，xxx_all，xxx_any。

**注意**：last_xxx 如果不满足，则会产生 `EnvError`，并在以后某个时间尝试重试。

### Executor

`Executor` 目前有三种形态：转换器、前置过滤器、后置过滤器。其配置都采用下面的形式。

```json
{
    "protocol": "http",						// 通讯协议，见下面的说明。
    "url": "http://my-executor/fun",		// 用于定位`Executor`的位置
    "settings": "executor self settings",	// 见下面的说明。
}
```

**protocol**： Nature 与 `Executor` 间的通讯协议，其值不区分大小写，目前支持下面的方式。

- Http | Https：通过 post 方式远程调用一个`Executor`。
- LocalRust：`Executor` 被实现为一个 Rust 的类库，Nature 通过 FFI 方式与该类库交互。
- Auto：当您不指定`executor`时，Nature在 `runtime` 会自动构建一个`executor`， 但`auto-executor`没有能力为 `Instance.content` 生成内容。所以当我们只关心ID、状态等信息时 Nature 的 `auto-executor` 会为我们带来很多便利。
- BuiltIn：使用Nature 内置的转换器进行转换。通过 `url` 属性来指定一个要使用的`builtin-executor`

http及LocalRust两种形式都需要您自行实现，请参考[Executor接口定义](executor.md)。

**settings**: 每个`Executor`可以有自己独立的的配置，这个配置由 Executor 自己进行解释。**注意** settings 的内容可以在 `runtime` 被 `Instance.sys_context`的`para.dynamic` 属性中的内容替换掉，而这种替换只局限于当前 Instance，不会影响到其它 Instance 。举例： 假设一个用于批量加载  Instance 的 before_filter 的 settings 配置如下：

```json
{
    "key_gt":"B:sale/item/(item_id):1|",
    "key_lt":"B:sale/item/(item_id):2|"
}
```

我们希望(item_id)在 `runtime` 被真正的ID 所替换。此时如果上游 `instance.sys_context`的 `para.dynamic` 属性中含有下面的定义，我们的愿望就可以实现了：

```properties
para.dynamic = "[[\"(item_id)\":\"123\"]]"
```

既在 Nature 调用 `Executor` 之前，会将 `settings`替换成下面的内容并传递给 `Exexutor`

```json
{
    "key_gt":"B:sale/item/123:1|",
    "key_lt":"B:sale/item/123:2|"
}
```

**注意**：目前 `para.dynamic` 只支持简单的替换，建议添加明确的边界符，如本示例用"()"，以避免发生错误的替换。

**`Executor`的一些示例**

```json
{
    "protocol":"Http",
    "url":"http://some_domain:8081/some_converter"
}
```

```json
{
    "protocol":"LocalRust",
    "url":"some_lib:some_converter"
}
```

```json
{
    "protocol":"builtIn",
    "url":"sum"
}
```

### 干预下游

在 `Executor` 执行完成后，有时我们会想附加一些信息到目标 `Instance` 上。比如，对于初始 `Order`我们可以将`OrderState`的状态自动置为 `new` 而不需要编程来实现。这时候我们可以通过下面的配置对结果进行干预。

```json
{
    "state_add": ["s1"],		// 默认 null, 在上个状态基础上增加新的状态，状态必须在 `Meta` 中定义过。
    "state_remove": ["s1"],		// 默认 null, 从上个状态中删除指定的状态。
    "append_para": [2,1],		// 默认 null, 从上游 Instance.para 中选取一部分附加到下游的 Instance.para 上，详细说明见下。
    "dynamic_para": "(key)",	 见下面说明
}
```

**append_para**：该属性说明如何生成下游 `Instance.para` 属性。示例，如其值为[3,1]， 假设上游 `Instance.para`为 “a/b/c/d”，则下游实例的 `para` 值为 “d/b”。如果下游 `para` 已经有值， 则在此值的后面附加。**注意**下游 `Meta` 如果是状态数据则自身 **para**  不能有值，否则无法形成版本数据。

**dynamic_para**: Nature 会用此生成  `Instance.sys_context` 属性的 `para.dynamic`，其格式如下：

```json
{"para.dynamic":"[[\"key\",\"value\"]]"}
```

其中的 key 来源于 `dynamic_para`  对应的值，而 value 则来源于 `append_para` 生成的附加值。`para.dynamic` 的作用为替换`Executor.settings`中的变量，请参考 [Demo](../../../nature-demo/README.md) 中的销售统计。