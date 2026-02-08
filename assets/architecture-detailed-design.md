# 全异步链路系统 - 详细设计文档

## 🎯 核心设计要点

### **不阻塞Primary的设计原则**
- **Primary路径绝对优先**：所有异步操作不得影响Primary响应时间
- **资源隔离**：Secondary处理使用专用线程池
- **错误隔离**：Secondary故障不得传播到Primary
- **背压控制**：异步操作要有合理的资源限制

---

## 🔧 Filter详细设计

### **4个核心Filter架构**

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
**职责**：双轨运行编排
**关键特性**：
- 根据运行模式决定路径（DUAL_RUN/SINGLE_RUN）
- Primary路径同步处理
- Secondary路径异步旁路处理
- 使用publish().autoConnect(2)确保body可重读

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
        
        // 关键：使用publish().autoConnect(2)创建共享流
        Flux<DataBuffer> sharedBody = exchange.getRequest().getBody()
            .publish().autoConnect(2); // 需要2个订阅者：Primary和Secondary
        
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

### **订阅者管理要点**
- **Primary订阅者**：chain.filter()自动订阅
- **Secondary订阅者**：processSecondaryAsync()手动订阅
- **背压控制**：autoConnect(2)确保只有2个订阅者
- **内存安全**：不缓存整个body，流式处理

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