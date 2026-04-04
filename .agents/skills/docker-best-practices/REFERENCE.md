# Docker Best Practices — Language Examples

## Go

```dockerfile
FROM golang:1.23.4-alpine AS base
RUN apk add --update gcc musl-dev
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY ./src ./src
RUN CGO_ENABLED=1 GOOS=linux go build -a -installsuffix cgo -o myapp ./src

FROM base AS test
RUN go install gotest.tools/gotestsum@v1.12.0
ENTRYPOINT ["gotestsum", "--jsonfile", "/out/tests.json", "--", "-coverprofile=/out/coverage.out", "./..."]

FROM alpine:3.21.0 AS artifact
RUN rm -f /sbin/apk && \
    rm -rf /etc/apk /lib/apk /usr/share/apk /var/lib/apk
COPY --from=base /app/myapp /home/nonroot/myapp
EXPOSE 8080
RUN addgroup --system nonroot && \
    adduser --system --ingroup nonroot nonroot
ENV HOME=/home/nonroot
RUN chown -R nonroot:nonroot $HOME && chown -R nonroot:nonroot /tmp
USER nonroot
WORKDIR /home/nonroot
ENTRYPOINT ["./myapp"]
```

## Node.js

```dockerfile
FROM node:22.12.0-alpine AS base
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --omit=dev
COPY ./src ./src
RUN npm run build

FROM base AS test
RUN npm ci
ENTRYPOINT ["npx", "jest", "--coverage", "--coverageDirectory=/out/coverage"]

FROM node:22.12.0-alpine AS artifact
RUN rm -f /sbin/apk && \
    rm -rf /etc/apk /lib/apk /usr/share/apk /var/lib/apk
WORKDIR /home/nonroot
COPY --from=base /app/dist ./dist
COPY --from=base /app/node_modules ./node_modules
EXPOSE 3000
RUN addgroup --system nonroot && \
    adduser --system --ingroup nonroot nonroot
ENV HOME=/home/nonroot
RUN chown -R nonroot:nonroot $HOME && chown -R nonroot:nonroot /tmp
USER nonroot
ENTRYPOINT ["node", "dist/index.js"]
```

## Python

```dockerfile
FROM python:3.13.1-slim AS base
WORKDIR /app
COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt
COPY ./src ./src
RUN python -m compileall src

FROM base AS test
COPY requirements-dev.txt ./
RUN pip install --no-cache-dir -r requirements-dev.txt
ENTRYPOINT ["pytest", "--cov=src", "--cov-report=xml:/out/coverage.xml", "src/"]

FROM python:3.13.1-slim AS artifact
WORKDIR /home/nonroot
COPY --from=base /app/src ./src
COPY --from=base /usr/local/lib/python3.13/site-packages /usr/local/lib/python3.13/site-packages
EXPOSE 8000
RUN groupadd --system nonroot && \
    useradd --system --gid nonroot nonroot
ENV HOME=/home/nonroot
RUN chown -R nonroot:nonroot $HOME && chown -R nonroot:nonroot /tmp
USER nonroot
ENTRYPOINT ["python", "-m", "src.main"]
```

## Java (Maven)

```dockerfile
FROM eclipse-temurin:21.0.5_11-jdk-alpine AS base
WORKDIR /app
COPY pom.xml ./
RUN mvn dependency:go-offline -B
COPY ./src ./src
RUN mvn package -DskipTests -B

FROM base AS test
ENTRYPOINT ["mvn", "test", "-B", \
  "-Djacoco.destFile=/out/jacoco.exec", \
  "-Dsurefire.reportsDirectory=/out/surefire-reports"]

FROM eclipse-temurin:21.0.5_11-jre-alpine AS artifact
RUN rm -f /sbin/apk && \
    rm -rf /etc/apk /lib/apk /usr/share/apk /var/lib/apk
WORKDIR /home/nonroot
COPY --from=base /app/target/myapp.jar ./myapp.jar
EXPOSE 8080
RUN addgroup --system nonroot && \
    adduser --system --ingroup nonroot nonroot
ENV HOME=/home/nonroot
RUN chown -R nonroot:nonroot $HOME && chown -R nonroot:nonroot /tmp
USER nonroot
ENTRYPOINT ["java", "-jar", "myapp.jar"]
```

## Rust

```dockerfile
FROM rust:1.83.0-alpine AS base
RUN apk add --update musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -f target/release/myapp
COPY ./src ./src
RUN cargo build --release

FROM base AS test
ENTRYPOINT ["sh", "-c", "cargo test 2>&1 | tee /out/test-results.txt"]

FROM alpine:3.21.0 AS artifact
RUN rm -f /sbin/apk && \
    rm -rf /etc/apk /lib/apk /usr/share/apk /var/lib/apk
COPY --from=base /app/target/release/myapp /home/nonroot/myapp
EXPOSE 8080
RUN addgroup --system nonroot && \
    adduser --system --ingroup nonroot nonroot
ENV HOME=/home/nonroot
RUN chown -R nonroot:nonroot $HOME && chown -R nonroot:nonroot /tmp
USER nonroot
WORKDIR /home/nonroot
ENTRYPOINT ["./myapp"]
```
