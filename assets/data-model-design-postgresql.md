# PostgreSQL数据模型设计文档

## 📋 文档概述

本文档提供了基于PostgreSQL的详细数据模型设计，充分利用PostgreSQL的高级特性，为双轨运行系统提供高效、可靠的数据存储方案。

## 🎯 设计目标

- **高性能存储**：支持高并发场景下的快速数据写入和查询
- **全链路追踪**：完整存储HTTP请求/响应的所有细节
- **智能对比**：高效存储和查询对比规则与结果
- **可扩展性**：支持未来业务需求的扩展
- **数据安全**：保护敏感信息，确保数据安全

## 📁 表结构设计

### 1. 核心表结构

#### 1.1 audit_request（入站请求审计）

| 字段名 | 数据类型 | 描述 | 索引 | 备注 |
|--------|----------|------|------|------|
| **id** | SERIAL | 数据层面主键 | 主键索引 | 内部使用 |
| **correlation_id** | VARCHAR(36) | 请求唯一标识 | 唯一索引 | 业务层面主键，UUID格式 |
| **channel_id** | VARCHAR(50) | 渠道标识 | 索引 | 如：mobile-app, web, api |
| **client_id** | VARCHAR(100) | 客户端标识 | 索引 | 如：ios-1.0, android-2.0 |
| **source_system_id** | VARCHAR(50) | 来源系统标识 | 索引 | 如：legacy-system, new-core |
| **source_system_instance** | VARCHAR(100) | 来源系统实例 | - | 如：legacy-prod-1, new-core-stage |
| **api_type** | VARCHAR(50) | API/消息类型 | 索引 | 如：payment.create |
| **arrival_time** | TIMESTAMPTZ | 请求到达时间 | BRIN索引 | 支持时区 |
| **http_method** | VARCHAR(10) | HTTP请求方法 | 索引 | GET/POST/PUT/DELETE |
| **request_path** | VARCHAR(255) | 请求URL路径 | 索引 | 如：/api/v1/payment |
| **query_params** | JSONB | URL查询参数 | GIN索引 | 键值对格式 |
| **client_ip** | VARCHAR(45) | 客户端IP地址 | 索引 | 支持IPv4/IPv6 |
| **user_agent** | VARCHAR(255) | 客户端User-Agent | - | - |
| **content_type** | VARCHAR(100) | 请求Content-Type | - | - |
| **content_length** | BIGINT | 请求体大小（字节） | - | - |
| **request_headers** | JSONB | 请求头部信息 | GIN索引 | 完整原始头部 |
| **request_headers_masked** | JSONB | 脱敏后的请求头 | GIN索引 | 敏感信息已处理 |
| **request_payload** | TEXT | 原始请求体 | - | TOAST存储，支持大payload |
| **payload_size** | BIGINT | 请求体大小（字节） | - | 用于监控和分析 |
| **payload_compressed** | BOOLEAN | 是否压缩 | - | TOAST压缩状态 |
| **request_payload_hash** | VARCHAR(64) | 请求体哈希值 | - | SHA-256 |
| **payload_storage_type** | VARCHAR(20) | 存储类型 | - | DATABASE/FILE_SYSTEM |
| **payload_file_path** | VARCHAR(255) | 文件系统路径 | - | 仅当存储类型为FILE_SYSTEM时使用 |
| **mode** | VARCHAR(20) | 运行模式 | 索引 | DUAL_RUN/NEW_ONLY |
| **metadata** | JSONB | 其他元数据 | GIN索引 | 动态扩展字段 |

#### 1.2 audit_response（核心系统处理结果）

| 字段名 | 数据类型 | 描述 | 索引 | 备注 |
|--------|----------|------|------|------|
| **id** | SERIAL | 自增主键 | 主键 | - |
| **correlation_id** | VARCHAR(36) | 请求唯一标识 | 索引 | 外键关联audit_request |
| **core_type** | VARCHAR(20) | 核心系统类型 | 联合索引 | LEGACY/NEW |
| **status** | VARCHAR(20) | 处理状态 | 索引 | SUCCESS/FAIL/TIMEOUT/SKIPPED |
| **http_status** | INTEGER | HTTP状态码 | 索引 | 200/404/500等 |
| **error_code** | VARCHAR(100) | 错误码 | - | - |
| **error_detail** | TEXT | 错误详情 | - | - |
| **latency_ms** | BIGINT | 处理延迟（毫秒） | - | - |
| **processing_start** | TIMESTAMPTZ | 处理开始时间 | - | 支持时区 |
| **processing_end** | TIMESTAMPTZ | 处理结束时间 | - | 支持时区 |
| **response_headers** | JSONB | 响应头部信息 | GIN索引 | 完整原始头部 |
| **response_headers_masked** | JSONB | 脱敏后的响应头 | GIN索引 | 敏感信息已处理 |
| **response_payload** | TEXT | 原始响应体 | - | TOAST存储，支持大payload |
| **response_payload_size** | BIGINT | 响应体大小（字节） | - | 用于监控和分析 |
| **response_payload_compressed** | BOOLEAN | 是否压缩 | - | TOAST压缩状态 |
| **response_payload_hash** | VARCHAR(64) | 响应体哈希值 | - | SHA-256 |
| **response_content_length** | BIGINT | 响应体大小（字节） | - | - |
| **response_payload_storage_type** | VARCHAR(20) | 存储类型 | - | DATABASE/FILE_SYSTEM |
| **response_payload_file_path** | VARCHAR(255) | 文件系统路径 | - | 仅当存储类型为FILE_SYSTEM时使用 |
| **api_type** | VARCHAR(50) | API/消息类型 | 索引 | 用于快速筛选和分组 |

#### 1.3 comparison_result（对比结果）

| 字段名 | 数据类型 | 描述 | 索引 | 备注 |
|--------|----------|------|------|------|
| **id** | SERIAL | 自增主键 | 主键 | - |
| **correlation_id** | VARCHAR(36) | 请求唯一标识 | 唯一索引 | 外键关联audit_request |
| **api_type** | VARCHAR(50) | API/消息类型 | 索引 | 如：payment.create |
| **equivalent** | BOOLEAN | 是否等价 | 索引 | TRUE/FALSE |
| **status** | VARCHAR(20) | 比对状态 | 索引 | SUCCESS/FAIL/TIMEOUT |
| **confidence** | DECIMAL(3,2) | 比对结果置信度 | - | 0.00-1.00 |
| **diff_summary** | TEXT | 差异摘要 | - | 差异的文字描述 |
| **diff_detail** | JSONB | 详细差异 | GIN索引 | path+legacyValue+newValue |
| **rule_version** | VARCHAR(20) | 使用的规则版本 | - | 如：v1.0 |
| **compare_time** | TIMESTAMPTZ | 对比时间 | BRIN索引 | 支持时区 |
| **legacy_latency_ms** | BIGINT | Legacy处理延迟 | - | 毫秒 |
| **new_core_latency_ms** | BIGINT | New Core处理延迟 | - | 毫秒 |
| **latency_difference_ms** | BIGINT | 延迟差异 | - | 毫秒 |

#### 1.4 comparison_rule（对比规则）

| 字段名 | 数据类型 | 描述 | 索引 | 备注 |
|--------|----------|------|------|------|
| **id** | SERIAL | 自增主键 | 主键 | - |
| **api_type** | VARCHAR(50) | API/消息类型 | 唯一索引 | 如：payment.create |
| **ignored_fields** | JSONB | 忽略字段列表 | GIN索引 | JSONPath格式 |
| **normalization** | JSONB | 归一化规则 | GIN索引 | 时间格式、数值精度等 |
| **enabled** | BOOLEAN | 是否启用 | 部分索引 | TRUE/FALSE |
| **version** | VARCHAR(20) | 规则版本 | - | 如：v1.0 |
| **created_at** | TIMESTAMPTZ | 创建时间 | - | 支持时区 |
| **updated_at** | TIMESTAMPTZ | 更新时间 | - | 支持时区 |
| **description** | TEXT | 规则描述 | - | 规则的详细说明 |

## ⚡ PostgreSQL高级特性利用

### 2. 分区表策略

#### 2.1 audit_request分区表

```sql
-- 创建主表
CREATE TABLE audit_request (
    id SERIAL PRIMARY KEY,
    correlation_id VARCHAR(36) UNIQUE NOT NULL,
    channel_id VARCHAR(50),
    client_id VARCHAR(100),
    source_system_id VARCHAR(50),
    source_system_instance VARCHAR(100),
    api_type VARCHAR(50),
    arrival_time TIMESTAMPTZ,
    http_method VARCHAR(10),
    request_path VARCHAR(255),
    query_params JSONB,
    client_ip VARCHAR(45),
    user_agent VARCHAR(255),
    content_type VARCHAR(100),
    content_length BIGINT,
    request_headers JSONB,
    request_headers_masked JSONB,
    request_payload TEXT,
    payload_size BIGINT,
    payload_compressed BOOLEAN DEFAULT TRUE,
    request_payload_hash VARCHAR(64),
    payload_storage_type VARCHAR(20) DEFAULT 'DATABASE',
    payload_file_path VARCHAR(255),
    mode VARCHAR(20),
    metadata JSONB
)
PARTITION BY RANGE (arrival_time);

-- 创建分区（按月）
CREATE TABLE audit_request_y2026m02 PARTITION OF audit_request
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');

CREATE TABLE audit_request_y2026m03 PARTITION OF audit_request
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');

-- 创建索引
CREATE INDEX idx_audit_request_api_type ON audit_request(api_type);
CREATE INDEX idx_audit_request_http_method ON audit_request(http_method);
CREATE INDEX idx_audit_request_mode ON audit_request(mode);

-- 新增字段索引
CREATE INDEX idx_audit_request_channel_id ON audit_request(channel_id);
CREATE INDEX idx_audit_request_client_id ON audit_request(client_id);
CREATE INDEX idx_audit_request_source_system ON audit_request(source_system_id);

-- JSONB索引
CREATE INDEX idx_audit_request_query_params ON audit_request USING GIN (query_params);

-- 表达式索引
CREATE INDEX idx_audit_request_user_agent ON audit_request USING btree ((request_headers->>'User-Agent'));
```

#### 2.2 audit_response分区表

```sql
-- 创建主表
CREATE TABLE audit_response (
    id SERIAL PRIMARY KEY,
    correlation_id VARCHAR(36),
    core_type VARCHAR(20),
    status VARCHAR(20),
    http_status INTEGER,
    error_code VARCHAR(100),
    error_detail TEXT,
    latency_ms BIGINT,
    processing_start TIMESTAMPTZ,
    processing_end TIMESTAMPTZ,
    response_headers JSONB,
    response_headers_masked JSONB,
    response_payload TEXT,
    response_payload_size BIGINT,
    response_payload_compressed BOOLEAN DEFAULT TRUE,
    response_payload_hash VARCHAR(64),
    response_content_length BIGINT,
    response_payload_storage_type VARCHAR(20) DEFAULT 'DATABASE',
    response_payload_file_path VARCHAR(255),
    api_type VARCHAR(50)
)
PARTITION BY RANGE (processing_end);

-- 创建分区（按月）
CREATE TABLE audit_response_y2026m02 PARTITION OF audit_response
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');

-- 创建索引
CREATE INDEX idx_audit_response_correlation_id ON audit_response(correlation_id);
CREATE INDEX idx_audit_response_core_type ON audit_response(core_type);
CREATE INDEX idx_audit_response_status ON audit_response(status);
CREATE INDEX idx_audit_response_http_status ON audit_response(http_status);
CREATE INDEX idx_audit_response_api_type ON audit_response(api_type);

-- 复合索引
CREATE INDEX idx_audit_response_correlation_core ON audit_response(correlation_id, core_type);
```

#### 2.3 comparison_result分区表

```sql
-- 创建主表
CREATE TABLE comparison_result (
    id SERIAL PRIMARY KEY,
    correlation_id VARCHAR(36) UNIQUE NOT NULL,
    api_type VARCHAR(50),
    equivalent BOOLEAN,
    status VARCHAR(20),
    confidence DECIMAL(3,2),
    diff_summary TEXT,
    diff_detail JSONB,
    rule_version VARCHAR(20),
    compare_time TIMESTAMPTZ,
    legacy_latency_ms BIGINT,
    new_core_latency_ms BIGINT,
    latency_difference_ms BIGINT
)
PARTITION BY RANGE (compare_time);

-- 创建分区（按月）
CREATE TABLE comparison_result_y2026m02 PARTITION OF comparison_result
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');

CREATE TABLE comparison_result_y2026m03 PARTITION OF comparison_result
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');

-- 创建索引
CREATE INDEX idx_comparison_result_api_type ON comparison_result(api_type);
CREATE INDEX idx_comparison_result_equivalent ON comparison_result(equivalent);
CREATE INDEX idx_comparison_result_status ON comparison_result(status);

-- JSONB索引
CREATE INDEX idx_comparison_result_diff_detail ON comparison_result USING GIN (diff_detail);
```

### 3. 高级索引策略

#### 3.1 BRIN索引（块范围索引）

```sql
-- 对时间字段使用BRIN索引
CREATE INDEX idx_audit_request_arrival_time ON audit_request USING brin (arrival_time);
CREATE INDEX idx_audit_response_processing_end ON audit_response USING brin (processing_end);
CREATE INDEX idx_comparison_result_compare_time ON comparison_result USING brin (compare_time);

-- BRIN索引优势：
-- 1. 索引大小仅为B树索引的1-5%
-- 2. 插入性能几乎不受影响
-- 3. 范围查询性能接近B树索引
```

#### 3.2 部分索引

```sql
-- 只为成功的请求创建索引
CREATE INDEX idx_audit_response_success ON audit_response(correlation_id) 
    WHERE status = 'SUCCESS';

-- 只为启用的规则创建索引
CREATE INDEX idx_comparison_rule_enabled ON comparison_rule(api_type) 
    WHERE enabled = TRUE;

-- 只为特定API类型创建索引
CREATE INDEX idx_audit_request_payment ON audit_request(correlation_id) 
    WHERE api_type LIKE 'payment.%';
```

#### 3.3 表达式索引

```sql
-- 对JSONB字段创建表达式索引
CREATE INDEX idx_audit_request_user_agent ON audit_request 
    USING btree ((request_headers->>'User-Agent'));

CREATE INDEX idx_audit_request_x_api_key ON audit_request 
    USING btree ((request_headers->>'X-API-Key'));

CREATE INDEX idx_audit_request_query_user_id ON audit_request 
    USING btree ((query_params->>'user_id'));

-- 对文本字段创建表达式索引
CREATE INDEX idx_audit_request_request_path_prefix ON audit_request 
    USING btree (substring(request_path FROM 1 FOR 50));
```

## 📊 数据存储策略

### 4. Payload存储策略

#### 4.1 存储策略定义（无对象存储）

| Payload大小 | 存储方式 | 适用场景 | 存储位置 |
|------------|----------|----------|----------|
| **小Payload** (< 1MB) | TEXT字段 | 大多数API请求/响应 | PostgreSQL TOAST |
| **大Payload** (1-10MB) | TEXT字段 | 较大的JSON/XML | PostgreSQL TOAST |
| **超大Payload** (> 10MB) | 文件系统 | 特大文件 | 本地文件系统 |

#### 4.2 PostgreSQL TOAST存储

```sql
-- 检查TOAST设置
SHOW default_toast_compression;

-- 设置为lz4压缩（更高性能）
ALTER TABLE audit_inbound 
ALTER COLUMN request_payload 
SET COMPRESSION lz4;

-- 存储Payload的示例
INSERT INTO audit_inbound (
    correlation_id, api_type, arrival_time, ...,
    request_payload, payload_size, request_payload_hash
) VALUES (
    'uuid-123', 'payment.create', NOW(), ...,
    '{"amount": 100, "currency": "USD"}',
    27,  -- payload大小（字节）
    '5eb63bbbe01eeed093cb22bb8f5acdc3'  -- 哈希值
);
```

#### 4.3 文件系统存储（仅超大Payload）

```java
public class FileSystemPayloadStorage {
    
    private final String basePath = "/data/integration/payloads";
    
    public String storeLargePayload(byte[] payload, String correlationId) {
        // 创建目录结构
        String dirPath = basePath + "/" + LocalDate.now().format(DateTimeFormatter.ofPattern("yyyy/MM/dd"));
        new File(dirPath).mkdirs();
        
        // 生成文件名
        String fileName = correlationId + "-" + System.currentTimeMillis() + ".bin";
        String fullPath = dirPath + "/" + fileName;
        
        // 写入文件
        try (FileOutputStream fos = new FileOutputStream(fullPath)) {
            fos.write(payload);
        }
        
        // 返回相对路径
        return fileName;
    }
    
    public byte[] retrievePayload(String fileName) {
        String fullPath = basePath + "/" + getPathFromFileName(fileName);
        return Files.readAllBytes(Paths.get(fullPath));
    }
}
```

### 5. 数据压缩策略

#### 5.1 TOAST存储优化

```sql
-- PostgreSQL自动对大字段使用TOAST存储
-- 验证TOAST设置
SELECT relname, reltoastrelid 
FROM pg_class 
WHERE relname IN ('audit_inbound', 'core_outcome');

-- TOAST存储优势：
-- 1. 自动压缩大字段
-- 2. 仅在需要时加载数据
-- 3. 支持外部分区存储
```

#### 5.2 压缩配置

```sql
-- 检查当前压缩设置
SHOW default_toast_compression;

-- 设置为pglz压缩（默认）
-- SET default_toast_compression = 'pglz';

-- 设置为lz4压缩（更高性能）
-- SET default_toast_compression = 'lz4';
```

## 🔒 数据安全策略

### 6. 数据脱敏策略

#### 6.1 敏感字段脱敏

| 敏感字段 | 脱敏方法 | 存储位置 |
|----------|----------|----------|
| **Authorization** | 哈希处理 | request_headers_masked |
| **Cookie** | 部分脱敏 | request_headers_masked |
| **X-API-Key** | 哈希处理 | request_headers_masked |
| **PII数据** | 脱敏/哈希 | request_payload_raw |
| **支付信息** | 脱敏处理 | request_payload_raw |

#### 6.2 脱敏实现示例

```sql
-- 脱敏函数示例
CREATE OR REPLACE FUNCTION mask_sensitive_data(data JSONB) RETURNS JSONB AS $$
DECLARE
    masked_data JSONB;
BEGIN
    -- 脱敏Authorization头
    masked_data := data;
    
    IF data ? 'Authorization' THEN
        masked_data := masked_data || jsonb_build_object('Authorization', '*** MASKED ***');
    END IF;
    
    -- 脱敏Cookie中的敏感信息
    IF data ? 'Cookie' THEN
        masked_data := masked_data || jsonb_build_object('Cookie', '*** MASKED ***');
    END IF;
    
    -- 脱敏X-API-Key
    IF data ? 'X-API-Key' THEN
        masked_data := masked_data || jsonb_build_object('X-API-Key', '*** MASKED ***');
    END IF;
    
    RETURN masked_data;
END;
$$ LANGUAGE plpgsql;

-- 使用脱敏函数
UPDATE audit_inbound 
SET request_headers_masked = mask_sensitive_data(request_headers)
WHERE correlation_id = 'uuid-123';
```

### 7. 访问控制策略

#### 7.1 角色权限设计

| 角色 | 权限 | 适用人员 |
|------|------|----------|
| **auditor** | 只读审计数据 | 审计人员 |
| **operator** | 读写核心数据 | 运维人员 |
| **admin** | 所有权限 | 管理员 |

#### 7.2 权限实现

```sql
-- 创建角色
CREATE ROLE auditor;
CREATE ROLE operator;
CREATE ROLE admin;

-- 授予权限
GRANT SELECT ON audit_inbound, core_outcome, comparison_result TO auditor;
GRANT SELECT, INSERT, UPDATE ON core_outcome, comparison_result TO operator;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO admin;

-- 创建用户并分配角色
CREATE USER audit_user WITH PASSWORD 'secure_password';
GRANT auditor TO audit_user;

CREATE USER ops_user WITH PASSWORD 'secure_password';
GRANT operator TO ops_user;

CREATE USER admin_user WITH PASSWORD 'secure_password';
GRANT admin TO admin_user;
```

## ⚡ 性能优化策略

### 8. 写入性能优化

#### 8.1 批量插入

```sql
-- 使用COPY命令批量导入
COPY audit_request (correlation_id, api_type, arrival_time, http_method, request_path, client_ip, request_payload, payload_size, request_payload_hash)
FROM '/path/to/audit_data.csv' CSV HEADER;

-- 使用INSERT INTO ... SELECT批量插入
INSERT INTO audit_response (correlation_id, core_type, status, http_status, latency_ms, response_payload, response_payload_size, response_payload_hash)
SELECT correlation_id, 'LEGACY', 'SUCCESS', 200, 100, response_data, LENGTH(response_data), hash(response_data)
FROM temp_audit_data
WHERE processed = TRUE;

-- 使用多值插入
INSERT INTO audit_request (correlation_id, api_type, arrival_time, request_payload, payload_size, request_payload_hash)
VALUES
('uuid-1', 'payment.create', NOW(), '{"amount": 100}', 15, 'hash1'),
('uuid-2', 'user.login', NOW(), '{"username": "test"}', 18, 'hash2'),
('uuid-3', 'order.create', NOW(), '{"items": []}', 13, 'hash3');

-- 使用id字段的高效查询
SELECT * FROM audit_request 
WHERE id > 1000 
ORDER BY id 
LIMIT 100;
```

#### 8.2 事务优化

```sql
-- 使用批量提交
BEGIN;

-- 执行多个插入操作
INSERT INTO audit_request (...) VALUES (...);
INSERT INTO audit_response (...) VALUES (...);
INSERT INTO audit_response (...) VALUES (...);

COMMIT;

-- 禁用自动提交（仅在批量操作时）
-- SET autocommit = OFF;
-- ... 批量操作 ...
-- COMMIT;
-- SET autocommit = ON;
```

#### 8.3 并行写入

```sql
-- 调整PostgreSQL参数以支持并行写入
-- 在postgresql.conf中设置：
-- max_worker_processes = 8
-- max_parallel_workers_per_gather = 4
-- maintenance_work_mem = 1GB

-- 监控并行度
SELECT * FROM pg_stat_activity WHERE state = 'active';
```

### 9. 查询性能优化

#### 9.1 TOAST存储优化

```sql
-- 分析表以更新统计信息
ANALYZE audit_request;
ANALYZE audit_response;
ANALYZE comparison_result;

-- 优化TOAST存储
-- 1. 选择合适的压缩算法
ALTER TABLE audit_request 
ALTER COLUMN request_payload 
SET COMPRESSION lz4;

ALTER TABLE audit_response 
ALTER COLUMN response_payload 
SET COMPRESSION lz4;

-- 2. 优化大字段查询
-- 避免SELECT *，只查询需要的字段
SELECT correlation_id, api_type, arrival_time 
FROM audit_request 
WHERE api_type = 'payment.create';

-- 3. 使用部分索引加速常用查询
CREATE INDEX idx_audit_request_payload_size_large ON audit_request(correlation_id)
WHERE payload_size > 1048576; -- 1MB

-- 查看查询计划
EXPLAIN ANALYZE 
SELECT * FROM audit_request 
WHERE api_type = 'payment.create' 
AND arrival_time > NOW() - INTERVAL '1 hour';

-- 强制使用索引
EXPLAIN ANALYZE 
SELECT * FROM audit_request 
WHERE api_type = 'payment.create' 
AND arrival_time > NOW() - INTERVAL '1 hour'
AND client_ip = '192.168.1.1';
```

#### 9.2 分区剪枝

```sql
-- 利用分区剪枝提高查询性能
EXPLAIN ANALYZE 
SELECT COUNT(*) FROM audit_request 
WHERE arrival_time BETWEEN '2026-02-01' AND '2026-02-02';

-- 分区剪枝会自动跳过不需要的分区
-- 执行计划会显示："Append (cost=0.00..100.00 rows=1000 width=8)"
-- 然后只扫描相关分区
```

#### 9.3 物化视图

```sql
-- 创建物化视图用于常用查询
CREATE MATERIALIZED VIEW mv_daily_stats AS
SELECT 
    DATE(arrival_time) AS date,
    api_type,
    http_method,
    COUNT(*) AS request_count,
    AVG(CASE WHEN core_type = 'LEGACY' THEN latency_ms END) AS avg_legacy_latency,
    AVG(CASE WHEN core_type = 'NEW' THEN latency_ms END) AS avg_new_core_latency
FROM audit_request a
LEFT JOIN audit_response c ON a.correlation_id = c.correlation_id
GROUP BY DATE(arrival_time), api_type, http_method;

-- 刷新物化视图
REFRESH MATERIALIZED VIEW mv_daily_stats;

-- 创建索引
CREATE INDEX idx_mv_daily_stats_date ON mv_daily_stats(date);
CREATE INDEX idx_mv_daily_stats_api_type ON mv_daily_stats(api_type);
```

## 📈 监控与维护

### 10. 存储监控

#### 10.1 表大小监控

```sql
-- 查看表大小
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(c.oid)) AS total_size,
    pg_size_pretty(pg_relation_size(c.oid)) AS table_size,
    pg_size_pretty(pg_indexes_size(c.oid)) AS index_size
FROM pg_class c
LEFT JOIN pg_namespace n ON n.oid = c.relnamespace
WHERE n.nspname = 'public'
AND c.relkind = 'r'
ORDER BY pg_total_relation_size(c.oid) DESC;

-- 查看分区表大小
SELECT 
    partitiontablename,
    pg_size_pretty(pg_total_relation_size(partitiontablename::regclass)) AS size
FROM pg_partitions
WHERE schemaname = 'public'
AND tablename = 'audit_request'
ORDER BY size DESC;
```

#### 10.2 TOAST使用监控

```sql
-- 查看TOAST表使用情况
SELECT 
    relname AS table_name,
    reltoastrelid,
    pg_size_pretty(pg_total_relation_size(reltoastrelid)) AS toast_size
FROM pg_class
WHERE reltoastrelid > 0
AND relkind = 'r'
AND relname IN ('audit_request', 'audit_response');

-- 查看TOAST压缩率
SELECT 
    attname,
    avg_width,
    n_distinct,
    null_frac
FROM pg_stats
WHERE tablename = 'audit_request'
AND attname IN ('request_payload', 'request_headers');
```

### 11. 数据维护

#### 11.1 VACUUM策略

```sql
-- 自动VACUUM设置
-- 在postgresql.conf中设置：
-- autovacuum = on
-- autovacuum_max_workers = 4
-- autovacuum_naptime = 10min
-- autovacuum_vacuum_threshold = 50
-- autovacuum_analyze_threshold = 50

-- 手动VACUUM（在低峰期）
VACUUM ANALYZE audit_request;
VACUUM ANALYZE audit_response;
VACUUM ANALYZE comparison_result;

-- 全量VACUUM（重建表，释放空间）
-- VACUUM FULL ANALYZE audit_request;
```

#### 11.2 数据清理策略

```sql
-- 按时间删除旧数据
DELETE FROM audit_request 
WHERE arrival_time < NOW() - INTERVAL '30 days';

-- 按分区删除
ALTER TABLE audit_request DROP PARTITION audit_request_y2026m01;

-- 归档策略
CREATE TABLE audit_request_archive AS
SELECT * FROM audit_request 
WHERE arrival_time < NOW() - INTERVAL '30 days';

-- 然后删除旧数据
DELETE FROM audit_request 
WHERE arrival_time < NOW() - INTERVAL '30 days';
```

## 🔮 扩展性设计

### 12. 未来扩展性

#### 12.1 字段扩展

| 扩展方向 | 实现方式 | 优势 |
|----------|----------|------|
| **动态元数据** | 使用JSONB字段 | 无需修改表结构 |
| **新业务字段** | 预留字段或JSONB | 灵活应对业务变化 |
| **跨系统集成** | 新增关联表 | 保持核心表结构稳定 |

#### 12.2 分片策略

```sql
-- 水平分片（未来扩展）
-- 按api_type范围分片
CREATE TABLE audit_request_payment PARTITION OF audit_request
    FOR VALUES FROM ('payment.') TO ('product.');

CREATE TABLE audit_request_product PARTITION OF audit_request
    FOR VALUES FROM ('product.') TO ('user.');

-- 按client_ip哈希分片
-- CREATE TABLE audit_request_shard_1 PARTITION OF audit_request
--     FOR VALUES WITH (MOD(client_ip_hash, 4) = 0);
```

## 📚 最佳实践

### 13. 开发最佳实践

#### 13.1 连接池配置

```yaml
# HikariCP连接池配置
spring:
  datasource:
    hikari:
      maximum-pool-size: 20
      minimum-idle: 5
      connection-timeout: 30000
      idle-timeout: 600000
      max-lifetime: 1800000
      pool-name: AuditPool
```

#### 13.2 批量操作

```java
// Java批量插入示例
public void batchInsertAuditRecords(List<AuditRecord> records) {
    jdbcTemplate.batchUpdate(
        "INSERT INTO audit_inbound (correlation_id, api_type, arrival_time, http_method, request_path, client_ip) " +
        "VALUES (?, ?, ?, ?, ?, ?)",
        new BatchPreparedStatementSetter() {
            @Override
            public void setValues(PreparedStatement ps, int i) throws SQLException {
                AuditRecord record = records.get(i);
                ps.setString(1, record.getCorrelationId());
                ps.setString(2, record.getApiType());
                ps.setTimestamp(3, Timestamp.from(record.getArrivalTime()));
                ps.setString(4, record.getHttpMethod());
                ps.setString(5, record.getRequestPath());
                ps.setString(6, record.getClientIp());
            }
            
            @Override
            public int getBatchSize() {
                return records.size();
            }
        }
    );
}
```

#### 13.3 异步处理

```java
// Spring异步处理示例
@Async("auditExecutor")
public CompletableFuture<Void> saveAuditRecord(AuditRecord record) {
    // 保存审计记录
    auditRepository.save(record);
    return CompletableFuture.completedFuture(null);
}

// 配置线程池
@Bean("auditExecutor")
public Executor auditExecutor() {
    ThreadPoolTaskExecutor executor = new ThreadPoolTaskExecutor();
    executor.setCorePoolSize(10);
    executor.setMaxPoolSize(50);
    executor.setQueueCapacity(1000);
    executor.setThreadNamePrefix("Audit-");
    executor.setRejectedExecutionHandler(new ThreadPoolExecutor.CallerRunsPolicy());
    executor.initialize();
    return executor;
}
```

### 14. 运维最佳实践

#### 14.1 备份策略

| 备份类型 | 频率 | 保留期 | 用途 |
|----------|------|--------|------|
| **全量备份** | 每日 | 7天 | 灾难恢复 |
| **增量备份** | 每小时 | 24小时 | 快速恢复 |
| **归档备份** | 每周 | 30天 | 历史查询 |

#### 14.2 性能调优

```sql
-- 调整PostgreSQL参数
-- 在postgresql.conf中设置：

-- 内存设置
shared_buffers = 4GB           # 总内存的25%
work_mem = 64MB               # 用于排序和哈希操作
maintenance_work_mem = 1GB    # 用于VACUUM等维护操作

-- 查询优化
random_page_cost = 1.1        # SSD存储设为1.1-1.3
effective_cache_size = 12GB   # 总内存的75%

-- 写入优化
wal_buffers = 16MB            # 事务日志缓冲区
max_wal_size = 2GB            # 最大WAL大小
min_wal_size = 80MB           # 最小WAL大小

-- 并行查询
max_parallel_workers = 8      # 并行工作进程数
max_parallel_workers_per_gather = 4  # 每个查询的最大并行度
```

#### 14.3 监控工具

| 监控工具 | 用途 | 配置建议 |
|----------|------|----------|
| **Prometheus** | 指标监控 | 采集PostgreSQL指标 |
| **Grafana** | 可视化面板 | 监控表大小、查询性能 |
| **pg_stat_statements** | 慢查询分析 | 启用并定期分析 |
| **pgBadger** | 日志分析 | 每日生成报告 |

## 🎉 总结

### 15. 设计优势

#### 15.1 PostgreSQL特性利用

- **分区表**：实现高效的数据管理和查询
- **JSONB支持**：灵活存储半结构化数据
- **高级索引**：BRIN、部分、表达式索引提升查询性能
- **TOAST存储**：自动压缩大字段，节省空间
- **事务一致性**：确保数据完整性

#### 15.2 性能优势

- **写入性能**：批量插入、并行写入支持高并发
- **查询性能**：多级索引策略加速各种查询场景
- **存储效率**：智能Payload存储策略，平衡性能和成本
- **维护成本**：分区表简化数据管理和备份

#### 15.3 安全优势

- **数据脱敏**：保护敏感信息
- **访问控制**：基于角色的精细权限管理
- **审计日志**：完整记录数据操作
- **数据加密**：支持传输和存储加密

### 16. 实施路线图

| 阶段 | 任务 | 时间 |
|------|------|------|
| **阶段一** | 创建核心表结构和基础索引 | 1周 |
| **阶段二** | 实现分区表策略和高级索引 | 1周 |
| **阶段三** | 实施大Payload对象存储策略 | 1周 |
| **阶段四** | 完善数据脱敏和安全策略 | 1周 |
| **阶段五** | 优化性能和监控系统 | 1周 |

### 17. 结论

本数据模型设计充分利用了PostgreSQL的高级特性，为双轨运行系统提供了一个**高性能、可靠、安全**的数据存储方案。通过合理的表结构设计、索引策略和存储优化，能够支持高并发场景下的快速数据处理，同时为未来业务需求的扩展预留了空间。

**设计特点**：
- 充分利用PostgreSQL高级特性
- 模块化、可扩展的表结构
- 精细的索引和分区策略
- 完善的数据安全和监控
- 详细的实施和维护指南

**适用场景**：
- 高并发API网关
- 全链路追踪系统
- 双轨运行迁移
- 实时数据对比分析

---

**文档版本**：v1.0
**最后更新**：2026-02-06
**编写者**：数据架构设计团队
