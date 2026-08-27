# Turning the architecture model into something to look at.
#
# The model beside this file is LikeC4 source. These targets render it: a single
# self-contained page to open or send to someone, a static site to host, and a
# dev server that reloads while the sources are edited.
#
# Paths are resolved from this file rather than from the working directory, so
# the targets behave the same whether they are run here, from the repository
# root with `make -C docs/architecture`, or through `-f`.
#
# Everything runs through `npx`, so nothing is installed into the workspace and
# there is no lockfile to keep current. The version is pinned here instead,
# because `@latest` would render a different page next month and this project
# does not accept output that varies with the day it was built. Override it to
# try a newer one:
#
#     make architecture LIKEC4_VERSION=1.60.0
#
# Output goes to `build/` beside the sources, ignored by `.gitignore`.
#
# This file is not part of the gate. `scripts/ci/gate.sh` is what decides
# whether a change is sound; these targets only produce documentation.

LIKEC4_VERSION ?= 1.59.2
LIKEC4 := npx --yes likec4@$(LIKEC4_VERSION)

MODEL := $(patsubst %/,%,$(dir $(abspath $(lastword $(MAKEFILE_LIST)))))

# Beside the sources it is built from, not in the repository's `target/`. Named
# `build` rather than `target` so it is never mistaken for cargo's, which is a
# different thing with a different lifetime.
OUT := $(MODEL)/build
PAGE := $(OUT)/index.html
SITE := $(OUT)/site
SOURCES := $(wildcard $(MODEL)/*.c4)

.DEFAULT_GOAL := help

.PHONY: help architecture architecture-site architecture-serve architecture-validate architecture-clean

help: ## List the targets
	@grep -hE '^[a-z][a-z-]*:.*## ' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN { FS = ":.*## " } { printf "  %-24s %s\n", $$1, $$2 }'

architecture: $(PAGE) ## Build one self-contained HTML page and print its path

# One file with every script, style and font inlined. It opens from the
# filesystem with no server and no network, which is what makes it something
# that can be sent to a reviewer rather than described to them.
$(PAGE): $(SOURCES)
	@mkdir -p $(OUT)
	$(LIKEC4) build $(MODEL) \
		--output-single-file \
		--output $(OUT) \
		--title 'adocpdf architecture'
	@echo
	@echo "Open $(PAGE)"

architecture-site: $(SOURCES) ## Build the static site, for hosting
	@mkdir -p $(SITE)
	$(LIKEC4) build $(MODEL) --output $(SITE) --title 'adocpdf architecture'
	@echo
	@echo "Serve $(SITE)"

architecture-serve: ## Serve the model on a dev server, reloading as it is edited
	$(LIKEC4) start $(MODEL)

architecture-validate: ## Parse the sources and lay out every view
	$(LIKEC4) validate $(MODEL)

architecture-clean: ## Remove the generated pages
	rm -rf $(OUT)
