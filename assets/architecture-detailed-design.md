# 🚀 全异步链路系统 - 详细设计文档

## 🎯 项目实现目标与架构优势

### **核心实现目标**
- **✅ 不阻塞Primary**：Secondary处理完全异步，确保Primary响应时间不受影响
- **✅ 双轨运行**：支持DUAL_RUN/SINGLE_RUN模式动态切换
- **✅ 异步记录**：完整记录请求生命周期，支持请求比对
- **✅ 生产就绪**：具备全链路追踪、安全审计、配置管理等生产级功能

### **架构技术优势**
- **🎯 反应式编程**：基于Spring Cloud Gateway + WebFlux + Reactor现代化技术栈
- **🎯 简洁优雅**：5个Filter架构，职责清晰，符合CLEAN/SOLID/DRY/KISS原则
- **🎯 高性能**：异步处理+资源隔离，支撑高并发场景
- **🎯 可观测性**：完整监控体系，支持分布式追踪和审计

### **完整的设计原则体系**

#### **1. 核心架构原则**
- **🎯 不阻塞Primary原则**：Secondary处理必须完全异步，不得影响Primary响应时间
- **🎯 错误隔离原则**：Secondary故障不得传播到Primary，确保系统稳定性
- **🎯 资源隔离原则**：异步操作使用专用线程池，避免资源竞争

#### **2. 软件工程原则**
- **✅ CLEAN架构**：层次清晰，依赖关系合理，业务逻辑与技术实现分离
- **✅ SOLID原则**：
  - **单一职责**：每个Filter/Component只负责一个明确功能
  - **开闭原则**：对扩展开放，对修改关闭
  - **里氏替换**：子类可以替换父类而不影响程序正确性
  - **接口隔离**：客户端不应该依赖它不需要的接口
  - **依赖倒置**：高层模块不依赖低层模块，都依赖抽象
- **✅ DRY原则**：避免重复代码，工具类复用，配置集中管理
- **✅ KISS原则**：简单优先，避免过度设计，明确技术选择

#### **3. 反应式编程原则**
- **⚡ 异步非阻塞**：使用Reactor的Mono/Flux处理异步操作
- **⚡ 背压控制**：合理控制数据流，避免内存溢出
- **⚡ 错误处理**：使用onErrorResume等操作符优雅处理异常
- **⚡ 上下文传递**：使用Reactor Context替代ThreadLocal

#### **4. 生产环境原则**
- **🔒 安全性原则**：数据脱敏，权限控制，审计日志
- **📊 可观测性原则**：完整监控，分布式追踪，性能指标
- **🔄 配置管理原则**：动态配置，版本控制，回滚机制
- **🚀 性能优化原则**：资源限制，缓存策略，连接池管理

#### **5. 团队协作原则**
- **👥 代码规范**：统一的代码风格和命名约定
- **📚 文档完整性**：代码即文档，API文档，架构文档
- **🧪 测试驱动**：单元测试，集成测试，性能测试
- **🔍 代码审查**：严格的代码审查流程和质量标准

---

## 🔧 Filter详细设计

### **🎯 5个核心Filter架构优势**

#### **架构简洁性优势**
- **✅ 职责单一**：每个Filter只负责一个明确的功能
- **✅ 执行顺序清晰**：通过@Order注解控制执行顺序
- **✅ 易于扩展**：新增Filter不影响现有逻辑
- **✅ 易于测试**：每个Filter可独立测试

#### **技术实现优势**
- **🎯 反应式编程**：基于WebFlux的GlobalFilter接口
- **🎯 异步处理**：支持Mono/Void返回类型
- **🎯 上下文传递**：使用Reactor Context替代MDC
- **🎯 错误处理**：完善的异常处理机制

### **5个核心Filter架构**

#### **1. CorrelationIdFilter (@Order(-1500))**
**职责**：分布式追踪ID管理
**关键特性**：
- 生成或传递Correlation ID
- 确保跨服务边界的ID一致性
- 添加到请求头和响应头

```java
@Component
@Order(-1500)
public class CorrelationIdFilter implements GlobalFilter {
    
    @Override
    public Mono<Void> filter(ServerWebExchange exchange, GatewayFilterChain chain) {
        String correlationId = exchange.getRequest().getHeaders().getFirst("X-Correlation-ID");
        if (correlationId == null || correlationId.isEmpty()) {
            correlationId = generateCorrelationId();
        }
        
        // 添加到请求头，传递给下游服务
        ServerHttpRequest mutatedRequest = exchange.getRequest().mutate()
            .header("X-Correlation-ID", correlationId)
            .build();
            
        ServerWebExchange mutatedExchange = exchange.mutate()
            .request(mutatedRequest)
            .build();
            
        // 添加到响应头，返回给客户端
        mutatedExchange.getResponse().getHeaders().add("X-Correlation-ID", correlationId);
        
        return chain.filter(mutatedExchange);
    }
    
    private String generateCorrelationId() {
        return UUID.randomUUID().toString().replace("-", "").substring(0, 16);
    }
}
```

#### **2. AuthFilter (@Order(-1000))**
**职责**：请求认证与鉴权
**关键特性**：
- JWT Token验证
- 基于角色的权限检查
- 认证失败立即返回错误，不继续后续Filter

```java
@Component
@Order(-1000)
public class AuthFilter implements GlobalFilter {
    
    private final AuthService authService;
    
    @Override
    public Mono<Void> filter(ServerWebExchange exchange, GatewayFilterChain chain) {
        String path = exchange.getRequest().getPath().value();
        
        // API级别的安全控制
        if (requiresAuthentication(path)) {
            return validateToken(exchange)
                .flatMap(valid -> {
                    if (valid) {
                        return validatePermission(exchange, path)
                            .flatMap(hasPermission -> {
                                if (hasPermission) {
                                    return chain.filter(exchange);
                                } else {
                                    return forbidden(exchange, "Insufficient permissions");
                                }
                            });
                    } else {
                        return unauthorized(exchange, "Invalid token");
                    }
                });
        }
        
        // 公开API直接通过
        return chain.filter(exchange);
    }
    
    private boolean requiresAuthentication(String path) {
        // 配置需要认证的API路径
        return path.startsWith("/api/v1/admin") || 
               path.startsWith("/api/v1/secure");
    }
    
    private Mono<Boolean> validateToken(ServerWebExchange exchange) {
        String token = extractToken(exchange.getRequest());
        return authService.validateToken(token)
            .onErrorReturn(false); // 错误时返回false
    }
    
    private Mono<Boolean> validatePermission(ServerWebExchange exchange, String path) {
        String token = extractToken(exchange.getRequest());
        String method = exchange.getRequest().getMethod().name();
        return authService.checkPermission(token, path, method)
            .onErrorReturn(false);
    }
    
    private Mono<Void> unauthorized(ServerWebExchange exchange, String message) {
        exchange.getResponse().setStatusCode(HttpStatus.UNAUTHORIZED);
        return exchange.getResponse().writeWith(
            Mono.just(exchange.getResponse().bufferFactory().wrap(message.getBytes()))
        );
    }
    
    private Mono<Void> forbidden(ServerWebExchange exchange, String message) {
        exchange.getResponse().setStatusCode(HttpStatus.FORBIDDEN);
        return exchange.getResponse().writeWith(
            Mono.just(exchange.getResponse().bufferFactory().wrap(message.getBytes()))
        );
    }
    
    private String extractToken(ServerHttpRequest request) {
        String authHeader = request.getHeaders().getFirst("Authorization");
        if (authHeader != null && authHeader.startsWith("Bearer ")) {
            return authHeader.substring(7);
        }
        return null;
    }
}
```

#### **2. DualRunFilter (@Order(-500))**  
**职责**：双轨运行编排 - **核心不阻塞Primary实现**
**关键特性**：
- **✅ 运行模式动态切换**：支持DUAL_RUN/SINGLE_RUN模式
- **✅ Primary路径绝对优先**：同步处理，确保响应时间
- **✅ Secondary路径完全异步**：使用专用线程池，不影响Primary
- **✅ 明确设计选择**：使用share()确保body可重读，大body不支持是性能优先的合理选择

**不阻塞Primary技术实现**：
```java
// Secondary处理完全异步，不阻塞Primary
if (isDualRunEnabled()) {
    processSecondaryAsync(sharedBody, correlationId).subscribe(); // 异步订阅
}
// Primary继续同步处理，不等待Secondary完成
return chain.filter(mutatedExchange); 
```

#### **3. AuditFilter (@Order(0))**
**职责**：审计记录
**关键特性**：
- 异步记录请求/响应数据
- 基于事件总线的松耦合设计
- 批量写入提升性能

#### **4. ResponseFilter (@Order(1000))**
**职责**：响应包装
**关键特性**：
- 添加Correlation ID等标准Header
- 异步发布响应事件
- 统一响应格式

### **Filter执行流程**
```
请求进入 → AuthFilter(认证) → DualRunFilter(路由) → AuditFilter(审计) → ResponseFilter(包装) → 响应返回
                                    ↓
                          Secondary异步处理（不阻塞）
```

---

## ⚡ 事件处理机制

### **事件类型设计**
```java
public enum EventType {
    REQUEST,    // 请求事件：包含请求元数据
    RESPONSE    // 响应事件：包含响应元数据
}
```

### **事件发布时机**
| 事件类型 | 发布时机 | 包含数据 |
|----------|----------|----------|
| REQUEST | 请求进入网关时 | correlationId, headers, path, method |
| RESPONSE | 响应返回客户端时 | correlationId, status, responseHeaders |

### **正确的事件总线实现（不阻塞Primary）**

```java
@Component
public class NonBlockingEventBus implements EventBus {
    
    // 关键：使用directBestEffort确保不阻塞发布者
    private final Sinks.Many<SystemEvent> eventSink = 
        Sinks.many().multicast().directBestEffort();
    
    private final EventMetrics metrics;
    
    @Override
    public Mono<Void> publish(SystemEvent event) {
        return Mono.fromRunnable(() -> {
            // 直接发布，不检查，不阻塞
            Sinks.EmitResult result = eventSink.tryEmitNext(event);
            
            if (!result.isSuccess()) {
                // 发布失败是设计预期的，记录指标即可
                metrics.recordEventDropped(result);
            }
        }).subscribeOn(Schedulers.boundedElastic()).then();
    }
    
    @Override
    public Flux<SystemEvent> getEventStream() {
        return eventSink.asFlux()
            .onBackpressureBuffer(100, BufferOverflowStrategy.DROP_OLDEST)
            .doOnNext(event -> metrics.recordEventProcessed())
            .doOnError(error -> metrics.recordEventError(error));
    }
}

### **简洁的监控指标设计**
```java
@Component
public class EventMetrics {
    
    private final MeterRegistry meterRegistry;
    
    // 核心监控指标
    private final Counter eventsPublished = Counter.builder("events.published").register(meterRegistry);
    private final Counter eventsProcessed = Counter.builder("events.processed").register(meterRegistry);
    private final Counter eventsDropped = Counter.builder("events.dropped").register(meterRegistry);
    private final Counter secondaryFailures = Counter.builder("secondary.failures").register(meterRegistry);
    
    public void recordEventPublished() {
        eventsPublished.increment();
    }
    
    public void recordEventProcessed() {
        eventsProcessed.increment();
    }
    
    public void recordEventDropped(Sinks.EmitResult result) {
        eventsDropped.increment();
        // 记录丢弃原因
        meterRegistry.counter("events.dropped.reason", "reason", result.name()).increment();
    }
    
    public void recordSecondaryFailure(String correlationId, Throwable error) {
        secondaryFailures.increment();
        // 记录错误类型
        meterRegistry.counter("secondary.failures.type", "type", error.getClass().getSimpleName()).increment();
    }
    
    public void recordEventError(Throwable error) {
        meterRegistry.counter("events.errors", "type", error.getClass().getSimpleName()).increment();
    }
}
}
```

### **事件处理器示例**
```java
@EventListener({EventType.REQUEST, EventType.RESPONSE})
public class AuditEventHandler implements EventHandler {
    
    @Override
    public Mono<Void> handle(SystemEvent event) {
        return auditService.recordEvent(event)
            .onErrorResume(error -> {
                log.error("Audit recording failed", error);
                return Mono.empty(); // 错误不影响主流程
            });
    }
}
```

---

## 🔄 Body复制与流式处理

### **Body复制挑战**
- **问题**：Spring WebFlux的DataBuffer只能被消费一次
- **解决方案**：使用publish().autoConnect(2)创建共享流，避免内存缓存

### **关键实现细节**
```java
@Component
@Order(-500)
public class DualRunFilter implements GlobalFilter {
    
    @Override
    public Mono<Void> filter(ServerWebExchange exchange, GatewayFilterChain chain) {
        String correlationId = generateCorrelationId();
        
        // 简化：使用share()创建共享流，明确大body不支持
        Flux<DataBuffer> sharedBody = exchange.getRequest().getBody().share();
        
        // 重新设置请求体
        ServerHttpRequest mutatedRequest = exchange.getRequest().mutate()
            .body(sharedBody)
            .build();
        ServerWebExchange mutatedExchange = exchange.mutate()
            .request(mutatedRequest)
            .build();
        
        // 异步处理Secondary
        if (isDualRunEnabled()) {
            processSecondaryAsync(sharedBody, correlationId).subscribe();
        }
        
        // 同步处理Primary
        return chain.filter(mutatedExchange);
    }
    
    private Mono<Void> processSecondaryAsync(Flux<DataBuffer> bodyStream, String correlationId) {
        return Mono.fromRunnable(() -> {
            bodyStream
                .collectList()
                .flatMap(buffers -> {
                    return webClient.post()
                        .uri(secondaryConfig.getBaseUrl())
                        .body(BodyInserters.fromDataBuffers(Flux.fromIterable(buffers)))
                        .exchangeToMono(response -> auditService.recordResponse(correlationId, response));
                })
                .timeout(Duration.ofSeconds(5)) // 超时控制：5秒
                .onErrorResume(error -> {
                    // 简洁的故障处理：记录错误，不影响Primary
                    log.warn("Secondary processing failed for correlationId: {}", correlationId, error);
                    metrics.recordSecondaryFailure(correlationId, error);
                    return Mono.empty();
                })
                .subscribeOn(Schedulers.boundedElastic())
                .subscribe();
        });
    }
}
```

### **简化设计原则**
- **大body不支持**：超过1MB的body直接跳过Secondary处理
- **简单优先**：使用`share()`替代复杂的订阅者管理
- **性能优先**：避免内存缓存，流式处理
- **明确限制**：Content-Length检查，超限直接拒绝

### **安全的Body处理方案**
```java
@Component
@Order(-500)
public class DualRunFilter implements GlobalFilter {
    
    @Override
    public Mono<Void> filter(ServerWebExchange exchange, GatewayFilterChain chain) {
        String correlationId = generateCorrelationId();
        
        // 1. 创建共享body流（使用publish().autoConnect(2)）
        Flux<DataBuffer> sharedBody = exchange.getRequest().getBody()
            .publish().autoConnect(2); // 需要2个订阅者：Primary和Secondary
        
        // 2. 重新设置请求体
        ServerHttpRequest mutatedRequest = exchange.getRequest().mutate()
            .body(sharedBody)
            .build();
        ServerWebExchange mutatedExchange = exchange.mutate()
            .request(mutatedRequest)
            .build();
        
        // 3. 异步处理Secondary（使用专用线程池）
        if (isDualRunEnabled()) {
            processSecondaryAsync(sharedBody, correlationId).subscribe();
        }
        
        // 4. 同步处理Primary（业务关键路径）
        return chain.filter(mutatedExchange);
    }
    
    private Mono<Void> processSecondaryAsync(Flux<DataBuffer> bodyStream, String correlationId) {
        return Mono.fromRunnable(() -> {
            bodyStream
                .collectList()
                .flatMap(buffers -> {
                    return webClient.post()
                        .uri(secondaryConfig.getBaseUrl())
                        .body(BodyInserters.fromDataBuffers(Flux.fromIterable(buffers)))
                        .exchangeToMono(response -> auditService.recordResponse(correlationId, response));
                })
                .subscribeOn(Schedulers.boundedElastic()) // 专用线程池
                .subscribe();
        });
    }
}
```

### **内存优化策略**
```java
@Component
public class BodySizeChecker {
    private static final long MAX_BODY_SIZE = 10 * 1024 * 1024; // 10MB
    
    public Mono<Boolean> isBodySizeAcceptable(ServerHttpRequest request) {
        return request.getBody()
            .reduce(0L, (total, buffer) -> total + buffer.readableByteCount())
            .map(size -> size <= MAX_BODY_SIZE);
    }
}
```

---

## 🏗️ 模块依赖与业务实现

### **核心模块划分**

#### **gateway模块**
- **职责**：纯技术网关，无业务逻辑
- **包含**：Filter实现、路由配置、WebClient配置

#### **runtime-orchestration模块**
- **职责**：核心业务编排
- **包含**：双轨运行逻辑、模式切换、服务发现

#### **request-tracing模块**  
- **职责**：全链路请求追踪
- **包含**：审计服务、事件处理、数据关联

#### **shared-infrastructure模块**
- **职责**：纯技术基础设施
- **包含**：事件总线、工具类、配置管理

## 🔗 全链路分布式追踪设计

### **追踪设计原则**
- **必须实现**：所有服务调用必须传递Correlation ID
- **全链路覆盖**：同步调用、异步调用、消息队列、数据库操作
- **数据一致性**：确保追踪数据的完整性和关联性
- **性能影响最小**：追踪操作不阻塞业务逻辑

### **核心追踪组件**

#### **1. CorrelationIdFilter (@Order(-1500))**
**职责**：全链路追踪ID管理
**关键特性**：
- 生成或传递Correlation ID
- 确保跨服务边界的ID一致性
- 添加到MDC确保日志追踪

```java
@Component
@Order(-1500)
public class CorrelationIdFilter implements GlobalFilter {
    
    @Override
    public Mono<Void> filter(ServerWebExchange exchange, GatewayFilterChain chain) {
        String correlationId = exchange.getRequest().getHeaders().getFirst("X-Correlation-ID");
        if (correlationId == null || correlationId.isEmpty()) {
            correlationId = generateCorrelationId();
        }
        
        String requestId = generateRequestId();
        String spanId = generateSpanId();
        
        // 必须添加到请求头，传递给所有下游服务
        ServerHttpRequest mutatedRequest = exchange.getRequest().mutate()
            .header("X-Correlation-ID", correlationId)
            .header("X-Request-ID", requestId) // 单个请求ID
            .header("X-Span-ID", spanId) // 调用链跨度ID
            .build();
            
        ServerWebExchange mutatedExchange = exchange.mutate()
            .request(mutatedRequest)
            .build();
            
        // 必须添加到响应头，返回给客户端
        mutatedExchange.getResponse().getHeaders().add("X-Correlation-ID", correlationId);
        
        // 使用Reactor Context传递追踪信息
        return chain.filter(mutatedExchange)
            .contextWrite(ctx -> ctx.put("correlationId", correlationId)
                                   .put("requestId", requestId)
                                   .put("spanId", spanId));
    }
    
    private String generateCorrelationId() {
        return UUID.randomUUID().toString().replace("-", "").substring(0, 16);
    }
    
    private String generateRequestId() {
        return UUID.randomUUID().toString().replace("-", "").substring(0, 8);
    }
    
    private String generateSpanId() {
        return UUID.randomUUID().toString().replace("-", "").substring(0, 8);
    }
}
```

#### **2. 下游服务追踪规范**
```java
// 下游服务必须实现的追踪拦截器
@Component
public class DownstreamTracingInterceptor implements ClientHttpRequestInterceptor {
    
    @Override
    public Mono<ClientHttpResponse> intercept(HttpRequest request, byte[] body, 
                                             ClientHttpRequestExecution execution) {
        
        return Mono.deferContextual(ctx -> {
            // 从Reactor Context获取追踪信息
            String correlationId = ctx.getOrDefault("correlationId", "");
            String parentSpanId = ctx.getOrDefault("spanId", "");
            String spanId = generateSpanId();
            
            if (!correlationId.isEmpty()) {
                // 传递追踪头到下游服务
                request.getHeaders().add("X-Correlation-ID", correlationId);
                request.getHeaders().add("X-Parent-Span-ID", parentSpanId);
                request.getHeaders().add("X-Span-ID", spanId);
            }
            
            // 记录调用开始
            long startTime = System.currentTimeMillis();
            
            return execution.execute(request, body)
                .doOnSuccess(response -> {
                    // 记录调用成功
                    long duration = System.currentTimeMillis() - startTime;
                    log.info("Downstream call completed: {} {}", request.getMethod(), request.getURI());
                    metrics.recordDownstreamCall(correlationId, request.getURI().toString(), duration, true);
                })
                .doOnError(error -> {
                    // 记录调用失败
                    long duration = System.currentTimeMillis() - startTime;
                    log.error("Downstream call failed: {} {}", request.getMethod(), request.getURI(), error);
                    metrics.recordDownstreamCall(correlationId, request.getURI().toString(), duration, false);
                });
        });
    }
}
```

#### **3. 异步调用追踪**
```java
@Component
public class AsyncTracingService {
    
    public <T> Mono<T> traceAsyncOperation(String operationName, Mono<T> operation) {
        String correlationId = MDC.get("correlationId");
        String spanId = generateSpanId();
        
        return Mono.deferContextual(ctx -> {
            // 在异步操作中保持追踪上下文
            MDC.put("correlationId", correlationId);
            MDC.put("spanId", spanId);
            
            long startTime = System.currentTimeMillis();
            
            return operation
                .doOnSuccess(result -> {
                    long duration = System.currentTimeMillis() - startTime;
                    log.info("Async operation completed: {}", operationName);
                    metrics.recordAsyncOperation(correlationId, operationName, duration, true);
                })
                .doOnError(error -> {
                    long duration = System.currentTimeMillis() - startTime;
                    log.error("Async operation failed: {}", operationName, error);
                    metrics.recordAsyncOperation(correlationId, operationName, duration, false);
                })
                .doFinally(signal -> {
                    MDC.clear();
                });
        });
    }
}
```

### **追踪数据存储和查询**

#### **1. 追踪数据模型**
```java
@Entity
@Table(name = "request_traces")
public class RequestTrace {
    
    @Id
    private String correlationId;
    
    private String requestId;
    private String spanId;
    private String parentSpanId;
    
    private String serviceName;
    private String operationName;
    private String httpMethod;
    private String requestPath;
    
    private LocalDateTime startTime;
    private LocalDateTime endTime;
    private Long duration;
    
    private Integer httpStatus;
    private Boolean success;
    private String errorMessage;
    
    @Column(columnDefinition = "JSONB")
    private String requestHeaders;
    
    @Column(columnDefinition = "JSONB")
    private String responseHeaders;
    
    // 索引优化
    @Index(name = "idx_correlation_id")
    private String correlationIdIndex;
    
    @Index(name = "idx_start_time")
    private LocalDateTime startTimeIndex;
}
```

#### **2. 追踪数据收集**
```java
@Component
public class TraceCollector {
    
    private final TraceRepository traceRepository;
    
    @Async
    public void collectTrace(RequestTrace trace) {
        // 异步保存追踪数据，不阻塞主流程
        traceRepository.save(trace)
            .onErrorResume(error -> {
                log.warn("Failed to save trace, but primary continues", error);
                return Mono.empty();
            })
            .subscribe();
    }
    
    public Flux<RequestTrace> findTracesByCorrelationId(String correlationId) {
        return traceRepository.findByCorrelationId(correlationId)
            .sort(Comparator.comparing(RequestTrace::getStartTime));
    }
    
    public Flux<RequestTrace> findTracesByTimeRange(LocalDateTime start, LocalDateTime end) {
        return traceRepository.findByStartTimeBetween(start, end)
            .take(1000); // 限制返回数量
    }
}
```

### **追踪配置规范**

#### **1. 下游服务配置**
```yaml
# 下游服务必须配置的追踪头
spring:
  cloud:
    gateway:
      default-filters:
        - CorrelationIdFilter
    
webclient:
  default-headers:
    X-Correlation-ID: "${correlationId}"
    X-Request-ID: "${requestId}"
    X-Span-ID: "${spanId}"

logging:
  pattern:
    level: "%5p [%X{correlationId}] %m%n"
```

#### **2. 追踪数据保留策略**
```yaml
tracing:
  retention:
    days: 30                    # 数据保留30天
    cleanup-interval: 3600      # 每小时清理一次
    batch-size: 1000            # 批量清理大小
  
  sampling:
    rate: 100%                  # 采样率（生产环境可调整为10%）
    enabled-apis: ["**"]        # 追踪所有API
    
  storage:
    type: "postgresql"          # 存储类型
    batch-size: 100             # 批量写入大小
    flush-interval: 1000        # 刷新间隔(ms)
```

### **追踪查询接口**

#### **1. 追踪查询API**
```java
@RestController
@RequestMapping("/api/v1/traces")
public class TraceController {
    
    private final TraceCollector traceCollector;
    
    @GetMapping("/{correlationId}")
    public Flux<RequestTrace> getTraceByCorrelationId(@PathVariable String correlationId) {
        return traceCollector.findTracesByCorrelationId(correlationId);
    }
    
    @GetMapping
    public Flux<RequestTrace> getTraces(@RequestParam @DateTimeFormat(iso = DateTimeFormat.ISO.DATE_TIME) LocalDateTime start,
                                       @RequestParam @DateTimeFormat(iso = DateTimeFormat.ISO.DATE_TIME) LocalDateTime end) {
        return traceCollector.findTracesByTimeRange(start, end);
    }
    
    @GetMapping("/search")
    public Flux<RequestTrace> searchTraces(@RequestParam String serviceName,
                                          @RequestParam(required = false) String operationName,
                                          @RequestParam(required = false) Boolean success) {
        return traceCollector.searchTraces(serviceName, operationName, success);
    }
}
```

## ⚙️ 生产环境配置验证和回滚机制

### **配置验证设计原则**
- **运行时验证**：配置变更时立即验证有效性
- **版本管理**：支持配置版本追踪和回滚
- **灰度发布**：配置变更可灰度发布
- **监控告警**：配置异常时自动告警

### **核心配置验证组件**

#### **1. 配置验证器**
```java
@Component
public class GatewayConfigValidator {
    
    private final GatewayConfig gatewayConfig;
    
    @EventListener
    public void onConfigRefresh(EnvironmentChangeEvent event) {
        // 配置变更时进行验证
        validateConfig();
    }
    
    public void validateConfig() {
        // 验证运行模式
        if (!isValidRunMode(gatewayConfig.getRunMode())) {
            throw new ConfigValidationException("Invalid run mode: " + gatewayConfig.getRunMode());
        }
        
        // 验证流量控制百分比
        if (!isValidPercentage(gatewayConfig.getSecondaryPercentage())) {
            throw new ConfigValidationException("Invalid secondary percentage: " + gatewayConfig.getSecondaryPercentage());
        }
        
        // 验证API路径配置
        if (!areValidApiPaths(gatewayConfig.getEnabledApis())) {
            throw new ConfigValidationException("Invalid API paths configuration");
        }
        
        log.info("Gateway configuration validated successfully");
    }
    
    private boolean isValidRunMode(String runMode) {
        return "DUAL_RUN".equals(runMode) || "SINGLE_RUN".equals(runMode);
    }
    
    private boolean isValidPercentage(String percentage) {
        try {
            int value = Integer.parseInt(percentage.replace("%", ""));
            return value >= 0 && value <= 100;
        } catch (NumberFormatException e) {
            return false;
        }
    }
    
    private boolean areValidApiPaths(List<String> apiPaths) {
        return apiPaths != null && apiPaths.stream().allMatch(path -> path.startsWith("/api/"));
    }
}
```

#### **2. 配置版本管理**
```java
@Component
public class ConfigVersionManager {
    
    private final ConfigRepository configRepository;
    
    @Value("${spring.application.name}")
    private String applicationName;
    
    public Mono<ConfigVersion> saveCurrentConfig() {
        ConfigVersion version = new ConfigVersion();
        version.setApplicationName(applicationName);
        version.setVersion(generateVersion());
        version.setConfigData(getCurrentConfigAsJson());
        version.setCreatedAt(LocalDateTime.now());
        
        return configRepository.save(version);
    }
    
    public Mono<ConfigVersion> rollbackToVersion(String version) {
        return configRepository.findByApplicationNameAndVersion(applicationName, version)
            .flatMap(configVersion -> {
                // 应用回滚配置
                return applyConfig(configVersion.getConfigData())
                    .thenReturn(configVersion);
            });
    }
    
    public Flux<ConfigVersion> getConfigHistory() {
        return configRepository.findByApplicationNameOrderByCreatedAtDesc(applicationName)
            .take(50); // 限制返回数量
    }
    
    private String generateVersion() {
        return "v" + LocalDateTime.now().format(DateTimeFormatter.ofPattern("yyyyMMddHHmmss"));
    }
}
```

#### **3. 配置变更监控**
```java
@Component
public class ConfigChangeMonitor {
    
    private final MeterRegistry meterRegistry;
    
    @EventListener
    public void onConfigChange(EnvironmentChangeEvent event) {
        // 记录配置变更指标
        meterRegistry.counter("config.change.count").increment();
        
        // 记录变更的配置项
        event.getKeys().forEach(key -> {
            log.info("Configuration changed: {}", key);
            metrics.recordConfigChange(key);
        });
    }
    
    @EventListener
    public void onConfigValidationError(ConfigValidationException exception) {
        // 配置验证失败告警
        log.error("Configuration validation failed", exception);
        alertService.sendAlert("配置验证失败", exception.getMessage());
    }
}
```

### **配置管理API**

#### **1. 配置管理接口**
```java
@RestController
@RequestMapping("/api/v1/config")
public class ConfigController {
    
    private final ConfigVersionManager configVersionManager;
    private final GatewayConfigValidator configValidator;
    
    @PostMapping("/version")
    public Mono<ConfigVersion> saveConfigVersion() {
        return configVersionManager.saveCurrentConfig();
    }
    
    @PostMapping("/rollback/{version}")
    public Mono<ConfigVersion> rollbackConfig(@PathVariable String version) {
        return configVersionManager.rollbackToVersion(version)
            .doOnSuccess(v -> log.info("Configuration rolled back to version: {}", version));
    }
    
    @GetMapping("/history")
    public Flux<ConfigVersion> getConfigHistory() {
        return configVersionManager.getConfigHistory();
    }
    
    @PostMapping("/validate")
    public Mono<Void> validateConfig() {
        return Mono.fromRunnable(configValidator::validateConfig);
    }
}
```

### **配置管理策略**

#### **1. 配置变更流程**
```yaml
config-change:
  workflow:
    - step: "validate"           # 验证配置
      timeout: "30s"
    - step: "save-version"       # 保存版本
      required: true
    - step: "notify"             # 通知相关方
      channels: ["slack", "email"]
    - step: "monitor"            # 监控变更影响
      duration: "5m"
```

#### **2. 回滚策略**
```yaml
rollback:
  triggers:
    - error-rate: "5%"           # 错误率超过5%
      timeout: "2m"
    - response-time: "2000ms"    # 响应时间超过2秒
      timeout: "1m"
    - manual: true               # 手动触发
  
  actions:
    - type: "rollback-config"
      target: "previous-version"
    - type: "notify"
      severity: "high"
```

## 🔒 安全审计和合规性设计

### **安全审计设计原则**
- **完整性**：记录所有关键操作和访问
- **不可篡改**：审计日志一旦生成不可修改
- **可追溯**：通过Correlation ID关联所有操作
- **合规性**：满足安全审计和合规要求

### **核心安全审计组件**

#### **1. 安全审计过滤器**
```java
@Component
@Order(100) // 在AuthFilter之后执行
public class SecurityAuditFilter implements GlobalFilter {
    
    private final SecurityAuditService auditService;
    
    @Override
    public Mono<Void> filter(ServerWebExchange exchange, GatewayFilterChain chain) {
        String path = exchange.getRequest().getPath().value();
        String method = exchange.getRequest().getMethod().name();
        
        // 记录请求开始
        return auditService.recordRequestStart(exchange)
            .then(chain.filter(exchange))
            .doOnSuccess(v -> {
                // 记录请求成功
                auditService.recordRequestSuccess(exchange);
            })
            .doOnError(error -> {
                // 记录请求失败
                auditService.recordRequestFailure(exchange, error);
            });
    }
}
```

#### **2. 安全审计服务**
```java
@Component
public class SecurityAuditService {
    
    private final AuditRepository auditRepository;
    
    public Mono<Void> recordRequestStart(ServerWebExchange exchange) {
        return Mono.deferContextual(ctx -> {
            String correlationId = ctx.getOrDefault("correlationId", "");
            
            AuditEvent event = new AuditEvent();
            event.setCorrelationId(correlationId);
            event.setEventType("REQUEST_START");
            event.setTimestamp(LocalDateTime.now());
            event.setHttpMethod(exchange.getRequest().getMethod().name());
            event.setRequestPath(exchange.getRequest().getPath().value());
            event.setClientIp(getClientIp(exchange));
            event.setUserAgent(exchange.getRequest().getHeaders().getFirst("User-Agent"));
            
            // 数据脱敏：敏感信息不记录
            event.setRequestHeaders(maskSensitiveHeaders(exchange.getRequest().getHeaders()));
            
            return auditRepository.save(event).then();
        });
    }
    
    public Mono<Void> recordRequestSuccess(ServerWebExchange exchange) {
        return Mono.deferContextual(ctx -> {
            String correlationId = ctx.getOrDefault("correlationId", "");
            
            AuditEvent event = new AuditEvent();
            event.setCorrelationId(correlationId);
            event.setEventType("REQUEST_SUCCESS");
            event.setTimestamp(LocalDateTime.now());
            event.setHttpStatus(exchange.getResponse().getStatusCode().value());
            event.setResponseTime(System.currentTimeMillis() - getRequestStartTime(exchange));
            
            return auditRepository.save(event).then();
        });
    }
    
    public Mono<Void> recordRequestFailure(ServerWebExchange exchange, Throwable error) {
        return Mono.deferContextual(ctx -> {
            String correlationId = ctx.getOrDefault("correlationId", "");
            
            AuditEvent event = new AuditEvent();
            event.setCorrelationId(correlationId);
            event.setEventType("REQUEST_FAILURE");
            event.setTimestamp(LocalDateTime.now());
            event.setErrorMessage(error.getMessage());
            event.setErrorType(error.getClass().getSimpleName());
            
            return auditRepository.save(event).then();
        });
    }
    
    private String maskSensitiveHeaders(HttpHeaders headers) {
        Map<String, String> maskedHeaders = new HashMap<>();
        
        headers.forEach((key, values) -> {
            if (isSensitiveHeader(key)) {
                maskedHeaders.put(key, "***");
            } else {
                maskedHeaders.put(key, String.join(",", values));
            }
        });
        
        return new JSONObject(maskedHeaders).toString();
    }
    
    private boolean isSensitiveHeader(String headerName) {
        return headerName.toLowerCase().contains("authorization") ||
               headerName.toLowerCase().contains("password") ||
               headerName.toLowerCase().contains("token");
    }
}
```

#### **3. 敏感操作审计**
```java
@Component
public class SensitiveOperationAuditor {
    
    private final SecurityAuditService auditService;
    
    public Mono<Void> auditSensitiveOperation(String operationType, String resource, 
                                             Map<String, Object> details) {
        return Mono.deferContextual(ctx -> {
            String correlationId = ctx.getOrDefault("correlationId", "");
            
            AuditEvent event = new AuditEvent();
            event.setCorrelationId(correlationId);
            event.setEventType("SENSITIVE_OPERATION");
            event.setTimestamp(LocalDateTime.now());
            event.setOperationType(operationType);
            event.setResource(resource);
            event.setOperationDetails(maskSensitiveData(details));
            
            return auditService.save(event).then();
        });
    }
    
    private String maskSensitiveData(Map<String, Object> data) {
        Map<String, Object> maskedData = new HashMap<>();
        
        data.forEach((key, value) -> {
            if (isSensitiveField(key)) {
                maskedData.put(key, "***");
            } else {
                maskedData.put(key, value);
            }
        });
        
        return new JSONObject(maskedData).toString();
    }
    
    private boolean isSensitiveField(String fieldName) {
        return fieldName.toLowerCase().contains("password") ||
               fieldName.toLowerCase().contains("token") ||
               fieldName.toLowerCase().contains("secret") ||
               fieldName.toLowerCase().contains("key");
    }
}
```

### **合规性检查**

#### **1. 合规性验证器**
```java
@Component
public class ComplianceValidator {
    
    public Mono<ComplianceResult> validateRequest(ServerWebExchange exchange) {
        return Mono.defer(() -> {
            ComplianceResult result = new ComplianceResult();
            
            // 检查请求头合规性
            if (!isHeadersCompliant(exchange.getRequest().getHeaders())) {
                result.addViolation("HEADER_COMPLIANCE", "Request headers violate compliance rules");
            }
            
            // 检查API访问合规性
            if (!isApiAccessCompliant(exchange.getRequest())) {
                result.addViolation("API_ACCESS_COMPLIANCE", "API access violates compliance rules");
            }
            
            // 检查数据保护合规性
            if (!isDataProtectionCompliant(exchange)) {
                result.addViolation("DATA_PROTECTION_COMPLIANCE", "Data protection compliance violated");
            }
            
            return Mono.just(result);
        });
    }
    
    private boolean isHeadersCompliant(HttpHeaders headers) {
        // 检查必要的安全头
        return headers.containsKey("X-Correlation-ID") &&
               headers.containsKey("User-Agent") &&
               !headers.containsKey("X-Forwarded-For"); // 防止IP伪造
    }
    
    private boolean isApiAccessCompliant(ServerHttpRequest request) {
        String path = request.getPath().value();
        String method = request.getMethod().name();
        
        // 检查API访问频率限制
        return isWithinRateLimit(path, method);
    }
    
    private boolean isDataProtectionCompliant(ServerWebExchange exchange) {
        // 检查数据保护合规性（如GDPR）
        return !containsSensitiveData(exchange);
    }
}
```

### **安全审计配置**

#### **1. 审计配置**
```yaml
security:
  audit:
    enabled: true
    retention-days: 365          # 审计日志保留1年
    sensitive-apis:              # 需要特别审计的API
      - "/api/v1/admin/**"
      - "/api/v1/secure/**"
      - "/api/v1/users/**"
    
    data-masking:                # 数据脱敏规则
      - pattern: "password=.*"
        replacement: "password=***"
      - pattern: "token=.*"
        replacement: "token=***"
      - pattern: "authorization: Bearer .*"
        replacement: "authorization: Bearer ***"
    
    compliance:
      gdpr: true                 # GDPR合规性检查
      pci-dss: false             # PCI DSS合规性检查
      hipaa: false               # HIPAA合规性检查
```

#### **2. 审计查询API**
```java
@RestController
@RequestMapping("/api/v1/audit")
public class AuditController {
    
    private final AuditRepository auditRepository;
    
    @GetMapping("/{correlationId}")
    public Flux<AuditEvent> getAuditEventsByCorrelationId(@PathVariable String correlationId) {
        return auditRepository.findByCorrelationId(correlationId)
            .sort(Comparator.comparing(AuditEvent::getTimestamp));
    }
    
    @GetMapping
    public Flux<AuditEvent> getAuditEvents(@RequestParam @DateTimeFormat(iso = DateTimeFormat.ISO.DATE_TIME) LocalDateTime start,
                                          @RequestParam @DateTimeFormat(iso = DateTimeFormat.ISO.DATE_TIME) LocalDateTime end) {
        return auditRepository.findByTimestampBetween(start, end)
            .take(1000); // 限制返回数量
    }
    
    @GetMapping("/sensitive")
    public Flux<AuditEvent> getSensitiveOperations(@RequestParam String operationType) {
        return auditRepository.findByEventTypeAndOperationType("SENSITIVE_OPERATION", operationType)
            .take(500);
    }
}
```

### **安全审计策略**

#### **1. 审计级别策略**
```yaml
audit-levels:
  low:
    events: ["REQUEST_START", "REQUEST_SUCCESS", "REQUEST_FAILURE"]
    retention: "30d"
    
  medium:
    events: ["SENSITIVE_OPERATION", "AUTH_FAILURE", "RATE_LIMIT_EXCEEDED"]
    retention: "90d"
    
  high:
    events: ["SECURITY_BREACH", "COMPLIANCE_VIOLATION", "DATA_LEAK"]
    retention: "365d"
```

#### **2. 告警策略**
```yaml
audit-alerts:
  security-breach:
    threshold: "1"              # 任何安全违规立即告警
    channels: ["slack", "email", "sms"]
    severity: "critical"
    
  compliance-violation:
    threshold: "5 per hour"     # 每小时5次合规性违规
    channels: ["slack", "email"]
    severity: "high"
    
  sensitive-operation:
    threshold: "100 per day"    # 每天100次敏感操作
    channels: ["slack"]
    severity: "medium"
```

### **业务实现要点**

#### **1. 双轨运行配置（支持动态切换）**
```yaml
gateway:
  run-mode: DUAL_RUN  # DUAL_RUN | SINGLE_RUN（支持热更新）
  primary:
    base-url: http://primary-service
    timeout: 5000
  secondary:
    base-url: http://secondary-service
    timeout: 3000
    enabled-apis: ["/api/v1/users", "/api/v1/orders"]  # API粒度控制
```

#### **2. 动态配置管理（支持流量控制和灰度发布）**
```yaml
# 支持流量控制和灰度发布的配置
gateway:
  run-mode: DUAL_RUN  # DUAL_RUN | SINGLE_RUN
  traffic-control:
    secondary:
      percentage: 10%    # 只有10%流量走Secondary
      enabled-apis: ["/api/v1/users", "/api/v1/orders"]
    canary:
      header: "X-Canary-Version"
      values: ["v2", "v3"]
      percentage: 5%
  primary:
    base-url: http://primary-service
    timeout: 5000
  secondary:
    base-url: http://secondary-service
    timeout: 3000
```

```java
@Component
@RefreshScope
public class GatewayConfig {
    
    @Value("${gateway.run-mode:DUAL_RUN}")
    private String runMode;
    
    @Value("${gateway.traffic-control.secondary.percentage:10%}")
    private String secondaryPercentage;
    
    @Value("${gateway.traffic-control.secondary.enabled-apis}")
    private List<String> enabledApis;
    
    @Value("${gateway.traffic-control.canary.percentage:5%}")
    private String canaryPercentage;
    
    private final Random random = new Random();
    
    public boolean isDualRunEnabled() {
        return "DUAL_RUN".equals(runMode);
    }
    
    public boolean isApiEnabled(String path) {
        return enabledApis.contains(path);
    }
    
    public boolean shouldRouteToSecondary() {
        // 流量控制：只有指定百分比的请求走Secondary
        int percentage = Integer.parseInt(secondaryPercentage.replace("%", ""));
        return random.nextInt(100) < percentage;
    }
    
    public boolean isCanaryRequest(ServerHttpRequest request) {
        // 灰度发布：基于Header判断
        String canaryHeader = request.getHeaders().getFirst("X-Canary-Version");
        return canaryHeader != null && !canaryHeader.isEmpty();
    }
}
```

#### **2. 事件处理器注册**
```java
@Configuration
public class EventHandlerConfig {
    
    @Bean
    @EventListener({EventType.REQUEST})
    public EventHandler metricsEventHandler() {
        return new MetricsEventHandler();
    }
    
    @Bean
    @EventListener({EventType.REQUEST, EventType.RESPONSE})
    public EventHandler auditEventHandler() {
        return new AuditEventHandler();
    }
}
```

#### **3. 错误处理策略**
```java
@Component
public class GlobalErrorHandler {
    
    @ExceptionHandler(Exception.class)
    public Mono<ServerResponse> handleException(Exception e) {
        // 记录错误但不影响Primary响应
        eventBus.publishErrorEvent(e).subscribe();
        
        return ServerResponse.status(HttpStatus.INTERNAL_SERVER_ERROR)
            .bodyValue(ErrorResponse.of("Internal Server Error"));
    }
}
```

---

## 📊 性能保障机制

### **性能基准指标**
```yaml
# 性能目标（单实例）
performance:
  primary:
    p99-latency: < 50ms     # Primary路径P99延迟
    qps: > 10,000           # 单机QPS
  secondary:
    impact: < 1ms           # Secondary对Primary的影响
    timeout: 5s             # Secondary超时时间
  memory:
    max-usage: 500MB        # 最大内存使用
    queue-size: 100         # 事件队列大小
```

### **线程池配置**
```java
@Configuration
public class ThreadPoolConfig {
    
    @Bean
    public Scheduler eventScheduler() {
        return Schedulers.newBoundedElastic(10, 100, "event");
    }
    
    @Bean
    public Scheduler secondaryScheduler() {
        return Schedulers.newBoundedElastic(5, 50, "secondary");
    }
}
```

### **背压控制**
- **事件处理**：有界队列防止内存溢出
- **Secondary转发**：连接超时和读取超时控制
- **数据库写入**：批量插入和异步处理

### **监控指标**
```yaml
management:
  endpoints:
    web:
      exposure:
        include: health,metrics,prometheus
  metrics:
    export:
      prometheus:
        enabled: true
```

---

## 🔧 部署与配置

### **环境配置**
```yaml
spring:
  application:
    name: async-link-gateway
  
server:
  port: 8080

logging:
  level:
    com.example.gateway: INFO
    reactor.netty: WARN
```

### **健康检查**
```yaml
management:
  endpoint:
    health:
      show-details: always
      probes:
        enabled: true
```

---

## 🚀 实施指南

### **开发阶段检查清单**
- [ ] Filter执行顺序正确配置
- [ ] 事件发布时机准确无误
- [ ] Secondary处理完全不阻塞Primary
- [ ] 错误处理覆盖所有异常场景

### **测试阶段检查清单**
- [ ] 单元测试覆盖所有Filter
- [ ] 集成测试验证双轨运行
- [ ] 性能测试达到目标指标
- [ ] 错误场景测试完整

### **生产部署检查清单**
- [ ] 线程池配置合理
- [ ] 监控告警配置生效
- [ ] 健康检查端点可用
- [ ] 日志收集配置正确

---

*本文档提供详细的技术实现指导，开发团队可基于此文档进行具体实现。*