
RUSTC=rustc

test:
	$(RUSTC) -Z debug-info -o appletest --test apple.rs
	./appletest
	

