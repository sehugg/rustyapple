
test:
	rustc -Z debug-info -o appletest --test apple.rs
	./appletest
	

