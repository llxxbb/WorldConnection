# Architecture

status data have versions to remark every change. all data history could be __traced__, and all data unchanged, even it is a status data.

she decouple business data from technology such as __service__, __Interface__ etc.,

如何实现幂等、一致性

retry

状态数据是如何实现的



只执行一次语义

Nature __short process__ can organize all the your business into a web directly,



one input and one output



## Status Data & Stateless Data

stateless data have only one version for an instance, but status data can have many version for a instance. If we look from outer, the stateless is immutable and status data is mutable.

例如 Order and OrderStatus.

## handle error

environment exception and business exception

## compatibility



## hot pluggable
