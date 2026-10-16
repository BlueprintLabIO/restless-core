# Patch the company browser before hosted release

Observed: immutable release 38916854059 qualified source a2209c4b, then its exact runtime-tools scan rejected pinned Chrome 155.0.8059.39 for fixed High/Critical vulnerabilities. Sealing and Cloud deployment did not run.

Layer: Runtime image and release operations. Keep the existing vulnerability gate and one canonical image recipe.

Acceptance: pin Chrome for Testing 155.0.8059.79 using independently downloaded SHA-256 values for linux64 and linux-arm64. Launch the AMD64 binary in an isolated existing Runtime filesystem, then build and qualify the exact canonical release image, pass the unchanged vulnerability scan, seal it and deploy Cloud. Upgrade the local supported release and company Runtime too. No ARM execution claim is inferred from download verification.

Makes deletable: the vulnerable browser pin. Temporary archives, extracted files, probe containers and any temporary builder are removed after acceptance.
