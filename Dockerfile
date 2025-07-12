# Dockerfile for Gemini Proxy Service
FROM rust:1.75-slim as builder

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    cmake \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 复制 Cargo 文件并构建依赖项（利用 Docker 层缓存）
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# 复制源代码
COPY src ./src
COPY config ./config

# 构建应用程序
RUN cargo build --release

# 运行时镜像
FROM debian:bookworm-slim@sha256:67f3931ad8cb1967beec602d8c0506af1e37e8d73c2a0b38b181ec5d8560d395

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# 创建应用用户
RUN groupadd -r gemini && useradd -r -g gemini -s /bin/false gemini

# 设置工作目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/gemini-proxy /usr/local/bin/gemini-proxy

# 复制配置文件
COPY config /app/config

# 创建必要的目录
RUN mkdir -p /app/logs /app/certs && \
    chown -R gemini:gemini /app

# 切换到应用用户
USER gemini

# 暴露端口
EXPOSE 8443 9443

# 健康检查 - 检查主服务和管理服务
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f -k https://localhost:8443/health || curl -f -k https://localhost:9443/health || exit 1

# 启动命令
CMD ["gemini-proxy"]