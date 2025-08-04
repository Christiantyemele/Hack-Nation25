module github.com/lognarrator/client

go 1.20

require (
	go.opentelemetry.io/collector v0.80.0
	go.opentelemetry.io/collector/component v0.80.0
	go.opentelemetry.io/collector/consumer v0.80.0
	go.opentelemetry.io/collector/exporter v0.80.0
	go.opentelemetry.io/collector/pdata v1.0.0-rc9
	go.opentelemetry.io/collector/processor v0.80.0
	go.opentelemetry.io/collector/receiver v0.80.0
	go.uber.org/zap v1.24.0
	gopkg.in/yaml.v2 v2.4.0
)

require (
	github.com/knadh/koanf v1.5.0
	github.com/spf13/cobra v1.7.0
)
