; Copyright (c) 2025 Erick Bourgeois, firestoned
; SPDX-License-Identifier: Apache-2.0
$TTL 3600
@       IN SOA  ns1.example.com. hostmaster.example.com. (
                2026100501 ; serial
                3600       ; refresh
                600        ; retry
                604800     ; expire
                86400 )    ; negative TTL
@       IN NS   ns1.example.com.
@       IN NS   ns2.example.com.
ns1     IN A    192.0.2.1
ns2     IN A    192.0.2.2
www     IN A    198.51.100.10
www     IN A    198.51.100.11
www     IN AAAA 2001:db8::10
api 300 IN A    203.0.113.5
docs    IN CNAME www.example.com.
@       IN MX   10 mail.example.com.
mail    IN A    198.51.100.25
_sip._tcp IN SRV 10 60 5060 sip.example.com.
sip     IN A    198.51.100.30
@       IN CAA  0 issue "letsencrypt.org"
; TXT last: hornet-bind9 0.1 dropped the record after a quoted TXT;
; regression-tested in tests/cli_tests.rs. Order is pinned by expected.*.
@       IN TXT  "v=spf1 mx -all"
