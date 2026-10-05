<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0

  GENERATED FILE: DO NOT EDIT.
  Source: calm/forage.architecture.json
  Regenerate with: make calm-docs
-->

# forage: named.conf to bindy CRDs

> Auto-generated from [`calm/forage.architecture.json`](https://github.com/firestoned/forage/blob/main/calm/forage.architecture.json)
> via `make calm-docs`. Edit the CALM model, not this page.

```mermaid
---
config:
  theme: base
  themeVariables:
    fontFamily: -apple-system, BlinkMacSystemFont, 'Segoe WPC', 'Segoe UI', system-ui, 'Ubuntu', sans-serif
    darkMode: false
    fontSize: 14px
    edgeLabelBackground: '#d5d7e1'
    lineColor: '#000000'
---
%%{init: {"layout": "dagre", "flowchart": {"htmlLabels": false}}}%%
flowchart TB
classDef boundary fill:#e1e4f0,stroke:#204485,stroke-dasharray: 5 4,stroke-width:1px,color:#000000;
classDef node fill:#eef1ff,stroke:#007dff,stroke-width:1px,color:#000000;
classDef iface fill:#f0f0f0,stroke:#b6b6b6,stroke-width:1px,font-size:10px,color:#000000;
classDef highlight fill:#fdf7ec,stroke:#f0c060,stroke-width:1px,color:#000000;

        subgraph bind9-host["BIND9 Host Filesystem"]
        direction TB
            named-conf["named.conf"]:::node
            zone-files["Zone Files"]:::node
        end
        class bind9-host boundary
        subgraph forage-cli["forage CLI"]
        direction TB
            mapper["Mapper"]:::node
            parsers["named.conf / Zone-File Parsers"]:::node
        end
        class forage-cli boundary

    bindy-crds["bindy CRDs"]:::node
    bindy-operator["bindy Operator"]:::node
    kubectl["kubectl"]:::node
    k8s-api["Kubernetes API Server"]:::node
    manifest-stream["Manifest Stream #40;stdout#41;"]:::node
    platform-operator["Platform Operator"]:::node

    platform-operator -->|The operator runs forage, reviews its output and applies it| forage-cli
    platform-operator -->|The operator runs forage, reviews its output and applies it| kubectl
    parsers -->|reads named.conf and includes #40;local file I/O, read-only#41;| named-conf
    parsers -->|reads zone files resolved from the file option #40;local file I/O, read-only#41;| zone-files
    mapper -->|maps the parsed AST| parsers
    mapper -->|prints manifests to stdout| manifest-stream
    kubectl -->|kubectl apply -f - #40;pipe or reviewed file#41;| manifest-stream
    kubectl -->|create/patch bindy CRs with the operator's credentials| k8s-api
    k8s-api -->|admission validates every manifest against the CRD schema| bindy-crds
    mapper -->|hand-written structs mirror the CRD wire format #40;ADR-0002#41;| bindy-crds
    bindy-operator -->|watches and reconciles the imported CRs| k8s-api



```
