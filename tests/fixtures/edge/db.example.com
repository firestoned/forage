; Copyright (c) 2025 Erick Bourgeois, firestoned
; SPDX-License-Identifier: Apache-2.0
$TTL 600
@       IN SOA  ns1.example.com. hostmaster.example.com. ( 2026100501 3600 600 604800 300 )
@       IN NS   ns1.example.com.
ns1     IN A    192.0.2.53
www.example.com. IN A 192.0.2.80
example.com.     IN MX 20 mx.example.com.
mx      IN A    192.0.2.25
alias   IN CNAME www
10      IN PTR  host.example.com.
; Long enough that the CR name is truncated to 253 characters. The cut must
; not land on a dot: that yields an invalid name (roadmap 01).
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.ccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc.dddddddddddddddddddddddddddddddddddddddd.tail IN A 192.0.2.99
