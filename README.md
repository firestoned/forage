# forage
Reads named.conf via hornet, parses zone files, emits DNSZone + record CRs (A, AAAA, CNAME, MX, TXT, SRV, CAA) as YAML to stdout. Multiple A/AAAA records for the same name are collapsed into a single CR with multiple addresses.
