stripintegrity:
	@echo "Stripping integrity hashes from the generated index file"
	perl -pi -e 's/ *?integrity *?= *?\".*?\"//' ./dist/index.html

build:
	@echo "Building"
	trunk build --release
	
builddebug:
	@echo "Building"
	trunk build
	
postbuild: stripintegrity

run: build postbuild
	
debug: builddebug postbuild

.DEFAULT_GOAL := run
