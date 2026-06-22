#!/usr/bin/env bash
# Download the DB-IP Lite GeoIP databases used by WireArch for offline
# country + ISP/ASN lookups. Licensed CC BY 4.0 by DB-IP (no account needed).
# Attribution required when displaying: "IP Geolocation by DB-IP" (https://db-ip.com).
#
# Usage: scripts/fetch-geoip.sh [target-dir]   (default: ./geoip)
set -euo pipefail

dir="${1:-geoip}"
mkdir -p "$dir"

for kind in country asn; do
    ok=0
    for offset in 0 1 2; do
        month="$(date -d "$(date +%Y-%m-01) -${offset} month" +%Y-%m)"
        url="https://download.db-ip.com/free/dbip-${kind}-lite-${month}.mmdb.gz"
        if curl -fsSL "$url" -o "$dir/dbip-${kind}-lite.mmdb.gz"; then
            gunzip -f "$dir/dbip-${kind}-lite.mmdb.gz"
            echo "Fetched dbip-${kind}-lite.mmdb (${month})"
            ok=1
            break
        fi
    done
    if [ "$ok" -ne 1 ]; then
        echo "Failed to fetch the ${kind} database" >&2
        exit 1
    fi
done

echo "GeoIP databases ready in ${dir}/"
echo "Attribution: IP Geolocation by DB-IP (https://db-ip.com)"
