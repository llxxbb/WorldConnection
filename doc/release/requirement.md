# 有待确认的需求

- 执行器：可以通过 context 来 申请ID
- context : executor setting 中可复制，可增加，可移除context 中的内容

    
- remove reqwest dependence, but actix_client not safe for multi-thread, need new for every time

开关：
    出于安全上的考虑，在处理过程中，动态路由不能回归到静态路由。


- use lru-cache for cache

- 修正 prepare.md
- write the document for summary use
- 流式计算：
  map filter sum fold groupBy reduce

- concurrent tasks limit and monitor
- redo check busy first

## Release 1.0.0

- 行使网关职能，充当所以业务系统的入口。
    - 查询接口，支持查询外系统接口：queryThird
    - Demo : 获取平均分（）
- 读取优化
    - Executor 支持内置缓存 适用于网关
    - 提供一致性哈希，以分散缓存的压力

- 支持第三方插件
    * monitor point
    * traffic limit

## Release 1.1.0


- 自路由：对`执行器`返回的自路由进行支持。
- ID 生成器 snowflake for 128 bits implement.
- builtin-executor sum :
    - for stream sum

## Release 2.0

- `Meta`：不同的`Meta` 的 `Instance` 有不同的生命周期
- `Meta`：支持属性验证，更新时验证是否有`Relation`在使用，如有则发出警告

## Release 3.0

- 提供后台管理界面
    - 更新配置时能够更新多个实例上的缓存
- `Meta`：支持属性验证，更新时验证是否有`Relation`在使用，如有则发出警告
- `Meta`：不同的`Meta` 的 `Instance` 有不同的生命周期
-  密文存储：简单方案，不区分用户，对称加密，公私钥方案见云版
- 分布式重试服务
- 文档：
    - 英文文档
    - `Instance` 的 `context` 说明。
    - 对 《概念》画一个流程图以说明怎样生成`Instance`
- 执行重试任务时，检测是否忙碌，如果忙碌，返回忙碌信息。
