# 全异步链路系统架构设计（优化版）

## 📋 文档导航

- **核心设计**：架构模式、技术选型、关键决策
- **功能实现**：双轨运行、审计追踪、结果对比
- **技术细节**：数据模型、部署架构、监控体系
- **项目管理**：实施路径、风险控制、成功指标
- **质量保障**：测试策略、安全考虑、最佳实践

## 🎯 核心设计要点与技术选型

### **架构核心** → **技术选型**
| 设计要点 | 技术实现 | 选型理由 |
|----------|----------|----------|
| **基于Spring Cloud Gateway的全异步架构** | Spring Cloud Gateway + WebFlux | 高性能网关，支撑高并发场景 |
| **单体多模块设计** | Spring Boot 3 + Maven多模块 | 平衡开发效率与系统性能 |
| **双轨运行模式** | 自定义过滤器 + Resilience4j | 实现Primary到Secondary的平滑迁移 |
| **事件驱动架构** | Project Reactor Sinks + 事件总线 | 松耦合设计，支持异步事件处理 |

### **技术关键** → **技术选型**
| 技术要点 | 技术实现 | 选型理由 |
|----------|----------|----------|
| **反应式编程** | WebFlux + Project Reactor | 非阻塞IO，资源利用率提升5-10倍 |
| **弹性设计** | Resilience4j + 熔断限流 | 系统稳定性保障，防止级联故障 |
| **可观测性** | Micrometer + Prometheus + ELK | 完整的监控、日志、追踪体系 |

### **业务价值** → **技术选型**
| 业务价值 | 技术实现 | 选型理由 |
|----------|----------|----------|
| **零影响迁移** | 线程池隔离 + 异步旁路 | 确保Primary系统不受影响 |
| **全链路审计** | 异步批量处理 + PostgreSQL | 请求全生命周期可追溯 |
| **智能对比** | 规则引擎 + JSON对比算法 | 规则驱动的差异分析 |
| **事件可观测性** | Sinks事件流 + 结构化日志 | 实时监控和故障排查 |

---

## 1. 设计目标与核心价值

### 1.1 核心目标
- **双轨运行**：实现Primary与Secondary的并行处理与结果对比
- **全链路审计**：确保请求全生命周期的可追溯性（raw payload、headers、metadata等）
- **高性能处理**：基于反应式架构支撑高并发场景
- **平滑迁移**：支持从Primary到Secondary的无缝切换
- **多实例HA**：支持水平扩展和跨实例调用关联
- **事件驱动**：基于Sinks的事件处理系统，实现松耦合架构

### 1.2 设计约束
- **记录不能阻塞业务**：审计/转发/outcome/diff的持久化失败不得影响请求处理，尤其不得影响Legacy
- **日志必须可检索**：关键流转产生结构化日志，包含Correlation ID、API类型、上下游标识等
- **数据安全**：对PII等敏感数据进行脱敏/哈希/归档

### 1.3 设计原则
- **异步非阻塞**：全链路采用反应式编程，最大化资源利用率
- **最小影响**：确保Legacy路径的稳定性和优先性
- **弹性设计**：内置限流、熔断、降级等弹性策略
- **可观测性**：完整的监控、日志和追踪体系
- **最小依赖**：简化部署和运维，降低复杂度

## 2. 架构概述

### 2.1 核心架构模式
```
客户端请求 → Spring Cloud Gateway → [双轨运行逻辑] → 响应返回
                                    ↓
                    [异步审计与对比处理]
```

### 2.2 运行模式
- **DUAL_RUN**：Primary（同步主路径）+ Secondary（异步旁路）
- **SINGLE_RUN**：仅Primary（同步主路径）

### 2.3 事件驱动架构
```
请求处理 → Filter链执行 → 事件发布 → 异步处理
    ↓
事件处理器 → 审计记录 → 结果对比 → 监控告警
```

## ⚡ 技术选型体系

### 3.1 技术选型决策矩阵

#### **核心选型标准**
- **性能优先**：支撑高并发，低延迟
- **生态成熟**：社区活跃，文档完善
- **运维友好**：部署简单，监控完善
- **扩展灵活**：为未来发展预留空间

#### **现代JVM技术趋势契合度**
| 现代趋势 | 当前选型 | 契合度 | 理由 |
|----------|----------|--------|------|
| **反应式编程** | WebFlux + Reactor | ⭐⭐⭐⭐⭐ | 非阻塞IO，资源利用率提升5-10倍 |
| **云原生架构** | Spring Boot 3 + 容器化 | ⭐⭐⭐⭐⭐ | 容器化部署，健康检查完善 |
| **微服务网关** | Spring Cloud Gateway | ⭐⭐⭐⭐⭐ | 高性能网关，内置弹性策略 |
| **反应式数据访问** | R2DBC | ⭐⭐⭐⭐⭐ | 原生反应式数据库访问 |
| **弹性设计** | Resilience4j | ⭐⭐⭐⭐⭐ | 轻量级，与WebFlux完美集成 |

#### **技术选型层次结构**
```
应用层
├── Spring Cloud Gateway（网关）
├── WebFlux + Reactor（反应式）
└── Resilience4j（弹性）

数据层
├── PostgreSQL（主存储）
├── Spring Data JPA（ORM框架）
└── R2DBC（反应式访问）

工具层
├── Jackson（JSON处理）
├── JsonPath（规则定义）
└── JSON Patch（差异生成）

运维层
├── 独立Jar包部署（无外部依赖）
└── 基础监控（可选，非必需）
```

### 3.2 核心技术组件详解

#### **3.2.1 网关层 - Spring Cloud Gateway**
**选型对比分析**：
| 网关技术 | 成熟度 | 性能 | Spring集成 | 当前选择 |
|----------|--------|------|------------|----------|
| **Spring Cloud Gateway** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ✅ **推荐** |
| Netflix Zuul | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | ❌ 维护中 |
| Kong/APISIX | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | ❌ 非JVM生态 |
| Nginx+Lua | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ | ❌ 开发复杂 |

**选型理由**：
- ✅ **性能基准**：基于Netty，单机QPS可达10万+
- ✅ **功能集成**：内置路由、过滤、限流、熔断
- ✅ **扩展机制**：支持自定义过滤器，业务适配灵活
- ✅ **生态整合**：与Spring Cloud体系无缝集成

**技术指标**：响应延迟<10ms，支持动态路由配置

#### **3.2.2 反应式核心 - WebFlux + Reactor**
**选型对比分析**：
| 反应式框架 | 成熟度 | 性能 | Spring集成 | 当前选择 |
|------------|--------|------|------------|----------|
| **WebFlux + Reactor** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ✅ **推荐** |
| Vert.x | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ❌ 集成复杂 |
| Akka | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | ❌ 学习曲线陡 |
| 传统Servlet | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ | ❌ 性能不足 |

**选型理由**：
- ✅ **资源效率**：线程利用率提升5-10倍，内存占用降低30%
- ✅ **背压机制**：自动流量控制，防止系统过载
- ✅ **响应性能**：P99延迟降低50%，用户体验显著提升

**适用场景**：QPS > 1000的高并发API网关

#### **3.2.3 弹性策略 - Resilience4j**
**选型对比**：
| 特性 | Resilience4j | Hystrix | 优势 |
|------|-------------|---------|------|
| 依赖 | 轻量级 | 较重 | 启动更快 |
| 反应式支持 | 原生支持 | 有限 | 与WebFlux完美集成 |
| 配置方式 | 代码+注解 | 主要注解 | 更灵活 |

#### **3.2.4 数据持久层 - Spring Data JPA + R2DBC**
**选型对比分析**：
| 数据访问技术 | 反应式支持 | 事务性 | 开发效率 | 当前选择 |
|--------------|------------|--------|----------|----------|
| **JPA + R2DBC** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ✅ **推荐** |
| 纯JPA/Hibernate | ❌ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⚠️ 阻塞式 |
| 纯R2DBC | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⚠️ 功能有限 |
| MyBatis | ❌ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ❌ 非反应式 |

**选型理由**：
- ✅ **开发效率**：Spring Data JPA简化CRUD操作，减少样板代码
- ✅ **反应式支持**：R2DBC提供反应式数据库访问，提升并发性能
- ✅ **事务一致性**：PostgreSQL ACID特性，数据可靠性保证
- ✅ **灵活切换**：支持阻塞式(JPA)和反应式(R2DBC)两种访问方式

### 3.3 JSON对比技术选型

#### **技术选型对比分析**
| 技术方案 | 成熟度 | 性能 | 功能完整性 | 当前选择 |
|----------|--------|------|------------|----------|
| **Jackson + 自定义引擎** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ✅ **推荐** |
| JsonUnit | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⚠️ 功能过重 |
| ZJSON | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ❌ 生态不成熟 |
| 纯自定义实现 | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | ❌ 开发成本高 |

#### **混合对比方案**
- **Jackson**：核心JSON处理，性能最优
- **JsonPath**：规则定义，支持字段忽略
- **JSON Patch**：标准差异生成，简洁高效

#### **对比流程**
1. **规则预处理**：使用JsonPath过滤忽略字段
2. **标准对比**：使用JSON Patch生成差异操作
3. **结果转换**：转换为业务友好的差异报告

#### **技术优势**
- ✅ **标准规范**：JSON Patch是RFC标准
- ✅ **功能完整**：支持忽略字段和自定义规则
- ✅ **性能优秀**：Jackson提供高性能JSON处理
- ✅ **部署简单**：纯Jar包依赖，无外部服务

## 🔑 关键设计决策

### 4.1 架构模式：单体多模块（核心决策）
**为什么选择单体多模块而非微服务？**

#### **决策依据**
- ✅ **功能耦合度高**：双轨运行、审计、对比逻辑紧密关联
- ✅ **性能最优**：进程内调用，无网络开销，响应延迟最低
- ✅ **团队规模适中**：无需引入分布式系统的复杂度
- ✅ **运维成本低**：单一应用部署，监控维护简单

#### **权衡分析**
| 方案 | 优势 | 劣势 | 适用场景 |
|------|------|------|----------|
| **单体多模块** | 性能最佳、开发简单 | 扩展性受限 | 团队<15人，功能耦合度高 |
| **微服务架构** | 扩展性强、技术异构 | 网络开销大、运维复杂 | 大型团队，功能独立性强 |

### **模块职责划分（清晰边界）**
- **gateway**：纯技术网关，无业务逻辑
- **runtime-orchestration**：核心业务编排（DUAL_RUN + NEW_ONLY）
- **request-tracing**：全链路请求追踪和审计
- **result-analysis**：结果对比分析和规则引擎
- **shared-infrastructure**：纯技术基础设施支持

### 4.2 异步处理策略
**核心原则**：主路径同步，旁路异步
- Legacy调用：同步处理，确保响应及时性
- New Core调用：异步旁路，避免阻塞主路径
- 审计记录：异步批量处理，提升吞吐量

### 4.3 弹性设计
- **限流**：基于内存的令牌桶算法（单机限流），防止系统过载
- **熔断**：监控服务健康状态，自动隔离故障服务
- **降级**：队列满时优雅降级，优先保证核心功能

## 5. 核心组件设计

### 5.1 Gateway配置
- **路由规则**：基于路径和Header的路由配置
- **过滤器链**：运行时编排、请求追踪、限流、熔断过滤器
- **服务发现**：集成Consul实现动态服务发现

### 5.2 运行时编排
- **DUAL_RUN模式**：同步转发Legacy，异步调用New Core
- **NEW_ONLY模式**：直接路由到New Core
- **模式切换**：支持运行时动态切换运行模式

### 5.3 请求追踪与结果分析
- **请求追踪**：全链路请求/响应记录，异步批量存储
- **结果分析**：规则驱动的异步对比分析，支持字段忽略和归一化
- **数据关联**：基于Correlation ID的全链路数据关联

## 6. 数据模型

### 6.1 详细数据模型设计

**完整的数据模型设计**已迁移到独立文档，充分利用PostgreSQL的高级特性：

- **详细表结构**：包含audit_inbound、core_outcome、comparison_result、comparison_rule四个核心表
- **PostgreSQL特性**：分区表、JSONB支持、高级索引策略
- **性能优化**：批量插入、并行写入、查询优化
- **数据安全**：脱敏策略、访问控制、审计日志

### 6.2 参考文档

**详细数据模型设计文档**：[PostgreSQL数据模型设计文档](data-model-design-postgresql.md)

### 6.3 核心表结构概览

| 表名 | 描述 | 主要字段 | 索引策略 |
|------|------|----------|----------|
| **audit_inbound** | 入站请求审计 | correlation_id, api_type, arrival_time, http_method, request_path, request_headers, request_payload_raw | 主键、BRIN、GIN、表达式索引 |
| **core_outcome** | 核心系统处理结果 | correlation_id, core_type, status, http_status, latency_ms, response_headers, response_payload_raw | 复合索引、状态索引、API类型索引 |
| **comparison_result** | 对比结果 | correlation_id, api_type, equivalent, diff_detail, legacy_latency_ms, new_core_latency_ms | 唯一索引、等价性索引、延迟差异索引 |
| **comparison_rule** | 对比规则 | api_type, ignored_fields, normalization, enabled, version | 唯一索引、部分索引（仅启用规则） |

### 6.4 关键设计要点

- **分区表策略**：按时间分区audit_inbound和core_outcome表
- **Payload存储**：小payload直接存储，大payload对象存储
- **索引优化**：BRIN索引（时间字段）、部分索引（常用查询）、表达式索引（JSONB字段）
- **数据压缩**：利用PostgreSQL TOAST存储自动压缩大字段
- **安全策略**：敏感信息脱敏、基于角色的访问控制

## 7. 部署与运维

### 7.1 部署架构

#### 7.1.1 极简部署（推荐）
- **独立Jar包部署**：单个可执行Jar包 + PostgreSQL
- **无外部依赖**：不依赖Redis、Consul、服务网格等
- **配置管理**：环境变量 + 配置文件，简单直接
- **容器化支持**：提供Dockerfile，支持Kubernetes部署

#### 7.1.2 扩展部署（高并发场景）
- **多实例部署**：基于CPU利用率和QPS自动缩放
- **数据库读写分离**：提升数据访问性能
- **对象存储**：MinIO/S3用于大文件存储

### 7.2 可观测性体系

#### 7.2.1 结构化日志
- **JSON格式**：使用Logback输出结构化JSON日志
- **关键字段**：correlationId、apiType、mode、status、latencyMs、upstream、downstream
- **日志级别**：ERROR/WARN/INFO/DEBUG，合理配置

#### 7.2.2 指标监控
- **核心指标**：
  - Gateway：QPS、响应时间P50/P90/P99、错误率、限流次数
  - Legacy：调用成功率、响应时间、错误率、熔断状态
  - New Core：调用成功率、响应时间、错误率、熔断状态
  - 批处理：批量大小、批处理延迟、写入失败率
- **监控集成**：Micrometer + Prometheus + Grafana

#### 7.2.3 健康检查
- **Spring Boot Actuator**：启用健康检查、指标、信息等端点
- **自定义健康检查**：为外部服务（Legacy、New Core、数据库）添加健康检查
- **Kubernetes探针**：配置就绪探针和存活探针

#### 7.2.4 告警策略
- **告警级别**：严重、警告、信息
- **告警阈值**：
  - 错误率 > 1%
  - P99响应时间 > 1s
  - 限流次数 > 100/min
  - 熔断状态变化

### 7.3 配置管理

#### 7.3.1 核心配置项
- **运行模式**：DUAL_RUN/NEW_ONLY
- **Correlation ID**：默认X-Correlation-Id
- **线程池配置**：核心线程数、最大线程数、队列大小
- **超时配置**：Legacy/New Core调用超时
- **重试策略**：New Core调用重试次数与退避策略
- **熔断配置**：失败率阈值、半开状态探测
- **限流配置**：QPS限制
- **降级策略**：队列满时的处理方式

#### 7.3.2 配置热更新
- **支持**：运行时动态调整线程池、超时、重试等配置
- **不支持**：运行模式切换需重启服务

## 8. 实施路径

### 8.1 阶段一：基础网关（2-3周）
- 部署Spring Cloud Gateway
- 实现基础路由和过滤器
- 建立监控和日志体系

### 8.2 阶段二：运行时编排（3-4周）
- 实现运行时编排逻辑
- 集成Legacy和New Core调用
- 完善弹性策略

### 8.3 阶段三：追踪分析（2-3周）
- 实现全链路请求追踪
- 开发结果分析功能
- 优化性能和稳定性

### 8.4 阶段四：生产验证（2-3周）
- 小流量验证
- 性能压测
- 正式上线

## 9. 成功指标

### 9.1 技术指标
- 系统可用性 ≥ 99.99%
- P99响应时间 < 1秒
- 错误率 < 0.1%

### 9.2 业务指标
- 运行时编排成功率 ≥ 99.9%
- 分析差异率逐步降低
- 迁移过程零故障

## 10. 风险与应对

### 10.1 技术风险
- **反应式编程学习曲线**：提前培训，渐进式实施
- **性能瓶颈**：充分的性能测试和优化
- **数据一致性**：完善的异常处理和重试机制

### 10.2 业务风险
- **Legacy影响**：严格的测试和监控
- **迁移失败**：完善的回滚策略
- **团队适应**：充分的技术支持和文档

## 🔧 核心实现细节

### 11.1 Filter执行顺序与职责

#### 11.1.1 基于@Order注解的Filter设计
| Order | Filter名称 | 职责 | 关键特性 |
|-------|------------|------|----------|
| -1000 | **AuthFilter** | 认证鉴权 | JWT验证、权限检查 |
| -500 | **RoutingFilter** | 路由分发 | Primary/Secondary路径选择 |
| 0 | **AuditFilter** | 审计记录 | 异步记录请求/响应数据 |
| 1000 | **ResponseFilter** | 响应包装 | 添加Correlation ID等Header |

#### 11.1.2 Filter实现示例
```java
@Component
@Order(-1000)
public class AuthFilter implements GlobalFilter {
    @Override
    public Mono<Void> filter(ServerWebExchange exchange, GatewayFilterChain chain) {
        // 认证逻辑
        return chain.filter(exchange);
    }
}

@Component
@Order(1000)
public class ResponseFilter implements GlobalFilter {
    @Override
    public Mono<Void> filter(ServerWebExchange exchange, GatewayFilterChain chain) {
        return chain.filter(exchange)
            .then(Mono.fromRunnable(() -> {
                // 响应包装逻辑
            }));
    }
}
```

### 11.2 事件驱动架构设计

#### 11.2.1 事件类型定义
```java
public enum EventType {
    REQUEST,      // 请求接收，用于记录请求数据
    RESPONSE      // 响应返回，用于记录响应数据
}
```

#### 11.2.2 事件发布时机
| 事件类型 | 发布时机 | 包含数据 |
|----------|----------|----------|
| REQUEST | 请求进入网关时 | correlationId, headers, path, method, payload |
| RESPONSE | 响应返回客户端时 | correlationId, status, latency, responseData |

#### 11.2.3 基于Sinks的事件总线
```java
@Component
public class EventBus {
    private final Sinks.Many<SystemEvent> eventSink = Sinks.many().multicast().directBestEffort();
    
    public Flux<SystemEvent> getEventStream() {
        return eventSink.asFlux();
    }
    
    public void publishEvent(SystemEvent event) {
        eventSink.tryEmitNext(event);
    }
}
```

### 11.3 异步链路实现细节

#### 11.3.1 双轨运行处理流程
```java
public Mono<ServerResponse> processDualRun(ServerRequest request) {
    String correlationId = generateCorrelationId();
    
    return processPrimary(request, correlationId)  // 同步Primary处理
        .doOnSuccess(response -> {
            // 异步Secondary处理，不阻塞主流程
            processSecondaryAsync(request, correlationId).subscribe();
        });
}
```

#### 11.3.2 背压控制与资源管理
- **线程池配置**：audit-pool（50线程，1000队列）
- **WebClient配置**：连接超时5s，读取超时10s
- **数据库连接池**：HikariCP（最大连接数20，最小连接数5）

## 12. 测试策略

### 12.1 测试层次

#### 12.1.1 单元测试
- **组件测试**：测试各组件的独立功能
- **过滤器测试**：测试自定义过滤器的逻辑
- **反应式测试**：使用StepVerifier测试反应式流
- **Mock外部服务**：使用WireMock模拟外部服务响应

#### 11.1.2 集成测试
- **端到端测试**：测试完整的请求处理流程
- **故障注入测试**：模拟网络延迟、服务不可用等故障场景
- **性能测试**：测试系统在高并发下的表现
- **限流测试**：测试限流机制的有效性
- **熔断测试**：测试熔断机制的有效性

#### 11.1.3 混沌测试
- **随机故障**：使用Chaos Monkey注入随机故障
- **恢复测试**：测试系统从故障中恢复的能力
- **容量测试**：测试系统的最大处理能力
- **网络分区测试**：测试网络分区场景下的系统行为

### 11.2 测试工具推荐
- **单元测试**：JUnit 5 + Mockito + WireMock
- **集成测试**：Testcontainers + WebTestClient
- **性能测试**：Gatling 或 JMeter
- **混沌测试**：Chaos Monkey for Spring Boot

## 12. 安全考虑

### 12.1 数据安全
- **敏感数据处理**：请求/响应中的PII数据脱敏或哈希处理
- **传输安全**：与Legacy/New Core的通信使用TLS
- **存储安全**：数据库加密、访问控制

### 12.2 访问控制
- **API认证**：与现有API Gateway集成
- **内部服务访问**：Kubernetes Service访问控制（如适用）
- **存储访问**：最小权限原则

### 12.3 安全审计
- **操作审计**：配置变更、服务重启等操作记录
- **异常检测**：监控异常请求模式、失败率突增等

### 12.4 安全最佳实践
- **依赖安全**：定期扫描依赖包漏洞
- **代码安全**：使用静态代码分析工具
- **容器安全**：Docker镜像安全扫描

---

**文档特点**：
- 聚焦核心设计理念，避免过度细节
- 突出关键决策和权衡
- 提供清晰的实施路径
- 强调风险控制和成功指标

**适用场景**：
- 技术决策和方案评审
- 团队技术培训和认知对齐
- 项目实施和进度跟踪

## 13. 结论与建议

### 13.1 总体结论

本架构设计文档提供了一个**现代、高效、可靠**的双轨运行系统解决方案：

- **技术先进性**：基于Spring Cloud Gateway和反应式编程，技术栈符合2023-2024年JVM生态最佳实践
- **实施可行性**：采用单体多模块设计，平衡了性能和开发效率，降低了实施复杂度
- **运维友好性**：极简部署方案，仅依赖PostgreSQL，大幅降低运维成本
- **业务价值**：实现了Legacy到New Core的零影响迁移，确保业务连续性

### 13.2 实施建议

1. **分阶段实施**：
   - **阶段一**：搭建基础网关，实现核心路由和过滤器
   - **阶段二**：实现双轨运行逻辑，集成Legacy和New Core
   - **阶段三**：完善审计和对比功能，建立监控体系
   - **阶段四**：小流量验证，性能压测，正式上线

2. **技术栈选择**：
   - 严格遵循文档中的技术选型，避免随意引入外部依赖
   - 优先使用Spring Boot和Spring Cloud生态组件
   - 保持代码风格和架构一致性

3. **团队准备**：
   - 组织反应式编程培训，熟悉WebFlux和Reactor
   - 建立代码审查和测试规范
   - 制定详细的监控和告警策略

## 14. 附录

### 14.1 术语定义

| 术语 | 解释 |
|------|------|
| **Legacy Core** | 现有的生产核心系统 |
| **New Core** | 新的核心系统 |
| **DUAL_RUN** | 双轨运行模式，同时调用Legacy和New Core |
| **NEW_ONLY** | 仅新核心运行模式，只调用New Core |
| **Correlation ID** | 请求的唯一标识，用于全链路追踪 |
| **WebFlux** | Spring的反应式Web框架 |
| **R2DBC** | 反应式关系型数据库客户端 |
| **Resilience4j** | 轻量级弹性策略库 |
| **JSON Patch** | RFC 6902标准，用于描述JSON文档的修改 |
| **JSONPath** | 用于在JSON文档中导航和提取数据的表达式语言 |

### 14.2 参考资料

- **Spring Cloud Gateway官方文档**：https://spring.io/projects/spring-cloud-gateway
- **Spring WebFlux官方文档**：https://docs.spring.io/spring-framework/docs/current/reference/html/web-reactive.html
- **Project Reactor官方文档**：https://projectreactor.io/docs/core/release/reference/
- **Resilience4j官方文档**：https://resilience4j.readme.io/docs
- **R2DBC官方文档**：https://r2dbc.io/
- **PostgreSQL官方文档**：https://www.postgresql.org/docs/

### 14.3 代码示例

#### 14.3.1 Correlation ID处理
```java
private String getCorrelationId(HttpHeaders headers) {
    String correlationId = headers.getFirst("X-Correlation-Id");
    if (correlationId == null) {
        correlationId = UUID.randomUUID().toString();
    }
    return correlationId;
}
```

#### 14.3.2 异步旁路处理
```java
@Async("newCoreExecutor")
public CompletableFuture<CoreOutcome> processNewCoreAsync(RequestContext context) {
    // 调用New Core
    // 记录outcome
    return CompletableFuture.completedFuture(outcome);
}
```

#### 14.3.3 JSON对比实现
```java
private JsonNode applyIgnoreRules(JsonNode node, List<String> ignorePaths) {
    // 使用JsonPath过滤忽略字段
    ObjectMapper mapper = new ObjectMapper();
    for (String path : ignorePaths) {
        node = JsonPath.parse(node.toString()).delete(path).json();
    }
    return node;
}

private JsonPatch generateDiff(JsonNode source, JsonNode target) {
    // 使用JSON Patch生成差异
    return JsonDiff.asJsonPatch(source, target);
}
```