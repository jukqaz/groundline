set -eu
umask 077
mkdir -p /tmp/groundline-dashboards
base64 -d /run/groundline/groundline-insights.json.gz.b64 > /tmp/groundline-dashboards/overview.gz
gzip -dc /tmp/groundline-dashboards/overview.gz > /tmp/groundline-dashboards/groundline-insights.json.new
mv /tmp/groundline-dashboards/groundline-insights.json.new /tmp/groundline-dashboards/groundline-insights.json
base64 -d /run/groundline/groundline-analysis.json.gz.b64 > /tmp/groundline-dashboards/analysis.gz
gzip -dc /tmp/groundline-dashboards/analysis.gz > /tmp/groundline-dashboards/groundline-analysis.json.new
mv /tmp/groundline-dashboards/groundline-analysis.json.new /tmp/groundline-dashboards/groundline-analysis.json
rm /tmp/groundline-dashboards/overview.gz /tmp/groundline-dashboards/analysis.gz
exec /run.sh
