# CI checks and release helpers.
#
#   make ci                      run every check (also runs on git push)
#   make version VERSION=x.y.z   bump, update CHANGELOG, run CI, commit and tag
#   make publish                 push main + tag, then publish to crates.io

CARGO   ?= cargo
REPO    := https://github.com/nuclearglow/shut
CURRENT := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
MSRV    := $(shell sed -n 's/^rust-version = "\(.*\)"/\1/p' Cargo.toml)
TODAY   := $(shell date +%F)

.DEFAULT_GOAL := help

.PHONY: help ci fmt fmt-check clippy test smoke msrv package \
	check-clean version tag publish

help:
	@echo "shut $(CURRENT) (MSRV $(MSRV))"
	@echo
	@echo "Checks:"
	@echo "  make ci                     fmt-check clippy test smoke msrv package"
	@echo "  make fmt                    format the code"
	@echo "  make fmt-check | clippy | test | smoke | msrv | package"
	@echo
	@echo "Release:"
	@echo "  make version VERSION=x.y.z  bump version, roll CHANGELOG, run ci, commit + tag"
	@echo "  make tag                    tag the current version (v$(CURRENT)) at HEAD"
	@echo "  make publish                run ci, push main + tag, cargo publish"

# --- checks -------------------------------------------------------------------

ci: fmt-check clippy test smoke msrv package
	@echo "ci: all checks passed"

fmt:
	$(CARGO) fmt

fmt-check:
	$(CARGO) fmt --check

clippy:
	$(CARGO) clippy --all-targets --locked -- -D warnings

test:
	$(CARGO) test --locked

# Nothing listens on port 1, so shut must exit with 69 (EX_UNAVAILABLE).
smoke:
	$(CARGO) run --quiet --locked -- --version
	@$(CARGO) run --quiet --locked -- 1; code=$$?; \
	if [ "$$code" -ne 69 ]; then echo "smoke: expected exit 69, got $$code"; exit 1; fi

msrv:
	@rustup toolchain list | grep -q '^$(MSRV)-' || { \
		echo "msrv: toolchain $(MSRV) missing; run: rustup toolchain install $(MSRV)"; exit 1; }
	$(CARGO) +$(MSRV) test --locked

package:
	$(CARGO) publish --dry-run --locked $(PACKAGE_FLAGS)

# --- release ------------------------------------------------------------------

check-clean:
	@git diff --quiet && git diff --cached --quiet || { \
		echo "working tree has uncommitted changes"; exit 1; }

# Moves everything under "## [Unreleased]" into a new "## [VERSION] - date"
# section and rewrites the compare links at the bottom of CHANGELOG.md.
version: check-clean
	@test -n "$(VERSION)" || { echo "usage: make version VERSION=x.y.z"; exit 1; }
	@echo "$(VERSION)" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$$' || { \
		echo "VERSION must be semver (x.y.z), got $(VERSION)"; exit 1; }
	@test "$(VERSION)" != "$(CURRENT)" || { echo "already at $(CURRENT)"; exit 1; }
	@! git rev-parse -q --verify "refs/tags/v$(VERSION)" >/dev/null || { \
		echo "tag v$(VERSION) already exists"; exit 1; }
	@grep -q '^## \[Unreleased\]' CHANGELOG.md || { \
		echo "CHANGELOG.md has no '## [Unreleased]' section"; exit 1; }
	@awk '/^## \[Unreleased\]/ { found = 1; next } \
		found && /^## / { exit } \
		found && NF { content = 1 } \
		END { exit !content }' CHANGELOG.md || { \
		echo "CHANGELOG.md: '## [Unreleased]' is empty; describe the changes first"; exit 1; }
	awk -v v="$(VERSION)" 'BEGIN { done = 0 } \
		!done && /^version = "/ { print "version = \"" v "\""; done = 1; next } \
		{ print }' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml
	awk -v v="$(VERSION)" -v prev="$(CURRENT)" -v d="$(TODAY)" -v repo="$(REPO)" \
		'/^## \[Unreleased\]/ { print; print ""; print "## [" v "] - " d; next } \
		/^\[Unreleased\]:/ { \
			print "[Unreleased]: " repo "/compare/v" v "...HEAD"; \
			print "[" v "]: " repo "/compare/v" prev "...v" v; next } \
		{ print }' CHANGELOG.md > CHANGELOG.md.tmp && mv CHANGELOG.md.tmp CHANGELOG.md
	$(CARGO) update --workspace
	@$(MAKE) --no-print-directory ci PACKAGE_FLAGS=--allow-dirty || { \
		echo "ci failed; inspect the bump, or undo it with: git checkout -- Cargo.toml Cargo.lock CHANGELOG.md"; exit 1; }
	git commit -am "chore(release): $(VERSION)"
	@$(MAKE) --no-print-directory tag
	@echo "next: make publish"

tag:
	@! git rev-parse -q --verify "refs/tags/v$(CURRENT)" >/dev/null || { \
		echo "tag v$(CURRENT) already exists"; exit 1; }
	git tag -a "v$(CURRENT)" -m "shut $(CURRENT)"

# Pushes before publishing: a failed push is harmless, a crates.io upload is not
# reversible.
publish: check-clean
	@test "$$(git branch --show-current)" = main || { \
		echo "publish from main (currently on $$(git branch --show-current))"; exit 1; }
	@test "$$(git rev-parse -q --verify 'refs/tags/v$(CURRENT)^{commit}')" = "$$(git rev-parse HEAD)" || { \
		echo "HEAD is not tagged v$(CURRENT); run: make tag"; exit 1; }
	@$(MAKE) --no-print-directory ci
	git push origin main "v$(CURRENT)"
	$(CARGO) publish --locked
