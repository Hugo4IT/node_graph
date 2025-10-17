#[macro_export]
macro_rules! connect {
    ($graph:expr; $($start:expr => $end:expr)+) => {
        $(
            $graph.connect($start, $end);
        )+
    };
}

#[macro_export]
macro_rules! connect_all {
    ($graph:expr; $($start:expr $(=> $end:expr)+;)+) => {
        $({
            let mut last = $start;

            $(
                $graph.connect(last, $end);
                last = $end;
            )+
        })+
    };
}
