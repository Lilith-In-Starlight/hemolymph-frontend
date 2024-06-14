stripintegrity:
	@echo "Stripping integrity hashes from the generated index file"
	perl -pi -e 's/ *?integrity *?= *?\".*?\"//' ./dist/index.html

build:
	@echo "Building"
	trunk build --release
	
postbuild: stripintegrity

run: build postbuild

.DEFAULT_GOAL := run
