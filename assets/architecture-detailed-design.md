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

#### **1. AuthFilter (@Order(-1000))**
**职责**：请求认证与鉴权
**关键特性**：
- JWT Token验证
- 基于角色的权限检查
- 认证失败立即返回错误，不继续后续Filter

#### **2. DualRunFilter (@Order(-500))**  
**职责**：双轨运行编排
**关键特性**：
- 根据运行模式决定路径（DUAL_RUN/SINGLE_RUN）
- Primary路径同步处理
- Secondary路径异步旁路处理
- 使用cache()操作符确保body可重读

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
            .doOnNext(event -> metrics.recordEventProcessed());
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

#### **1. 双轨运行配置**
```yaml
gateway:
  run-mode: DUAL_RUN  # DUAL_RUN | SINGLE_RUN
  primary:
    base-url: http://primary-service
    timeout: 5000
  secondary:
    base-url: http://secondary-service
    timeout: 3000
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