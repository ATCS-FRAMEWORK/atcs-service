ATCS Service
Motor de observabilidade cross-platform.
Uso
export OTLP_ENDPOINT="https://otlp-gateway-prod-sa-east-1.grafana.net/otlp"
./atcs-service
Docker
docker run -e OTLP_ENDPOINT="..." -e OTLP_HEADERS="..." atcs-service
