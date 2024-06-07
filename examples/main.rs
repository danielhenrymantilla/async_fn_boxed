use ::async_fn_boxed::async_fn_boxed;

#[async_fn_boxed]
async fn foo(s: &str) -> i32 {
    async {}.await;
    _ = (s,);
    42
}

#[async_fn_boxed]
async fn bar(_: &str) -> i32 {
    27
}

fn main() {
    _ = async {
        let same_types = [foo(""), bar("")];
        for future in same_types {
            let _: i32 = future.await;
        }
    }
}

#[async_fn_boxed]
pub async fn fibo(n: u64) -> u64 {
    if n <= 1 {
        n
    } else {
        fibo(n - 1).await + fibo(n - 2).await
    }
}
