
compile:
	cd c-blosc2
	rm -rf build
	rm -rf install
	mkdir build
	cd build
	cmake -DCMAKE_INSTALL_PREFIX=../install \
		-DCMAKE_C_FLAGS="-fPIE" \
		-DCMAKE_POSITION_INDEPENDENT_CODE=ON \
		-DBLOSC_INSTALL=ON \
		-DBUILD_TESTS=OFF \
		-DBUILD_EXAMPLES=OFF \
		-DBUILD_BENCHMARKS=OFF \
		-DBUILD_FUZZERS=OFF \
		-DSTATIC_LIB=ON \
		-DSHARED_LIB=ON \
		..
	cmake --build . --target install
	cd ../..
