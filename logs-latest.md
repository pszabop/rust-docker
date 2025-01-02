# Logs from 1.0.34:
```
11::09::58 dbug: Session[0] [172.28.0.2:47298] [02940EBE] Starting RespServerSession Id=1
11::09::58 trce: Session[0] [172.28.0.2:47298] [02940EBE] RECV: [*4|$6|CLIENT|$7|SETINFO|$8|LIB-NAME|$8|redis-rs|*4|$6|CLIENT|$7|SETINFO|$7|LIB-VER|$6|0.27.6|]
11::09::58 trce: Session[0] [172.28.0.2:47298] [02940EBE] SEND: [+OK|+OK|]
11::09::58 trce: Session[0] [172.28.0.2:47298] [02940EBE] RECV: [*3|$6|INCRBY|$3|foo|$1|1|]
11::09::58 trce: Session[0] [172.28.0.2:47298] [02940EBE] SEND: [:1|]
11::09::58 trce: Session[0] [172.28.0.2:47298] [02940EBE] RECV: [*3|$6|EXPIRE|$3|foo|$1|5|]
11::09::58 trce: Session[0] [172.28.0.2:47298] [02940EBE] SEND: [:1|]
11::10::00 trce: Session[0] [172.28.0.2:47298] [02940EBE] RECV: [*2|$3|GET|$3|foo|]
11::10::00 trce: Session[0] [172.28.0.2:47298] [02940EBE] SEND: [$1|1|]
11::10::05 trce: Session[0] [172.28.0.2:47298] [02940EBE] RECV: [*2|$3|GET|$3|foo|]
11::10::05 trce: Session[0] [172.28.0.2:47298] [02940EBE] SEND: [$-1|]
11::10::05 trce: Session[0] [172.28.0.2:47298] [02940EBE] RECV: [*3|$6|INCRBY|$3|foo|$1|1|]
11::10::05 trce: Session[0] [172.28.0.2:47298] [02940EBE] SEND: [:1|]
11::10::06 trce: Session[0] [172.28.0.2:47298] [02940EBE] RECV: [*2|$3|GET|$3|foo|]
11::10::06 trce: Session[0] [172.28.0.2:47298] [02940EBE] SEND: [$1|1|]
11::10::06 dbug: Session[0] [172.28.0.2:47298] [02940EBE] Disposing RespServerSession Id=1
```

# Logs from 1.033
```
11::46::44 trce: Session[0] [172.28.0.2:47706] [037A9C05] RECV: [*4|$6|CLIENT|$7|SETINFO|$8|LIB-NAME|$8|redis-rs|*4|$6|CLIENT|$7|SETINFO|$7|LIB-VER|$6|0.27.6|]
11::46::44 trce: Session[0] [172.28.0.2:47706] [037A9C05] SEND: [-ERR unknown command|-ERR unknown command|]
11::46::44 trce: Session[0] [172.28.0.2:47706] [037A9C05] RECV: [*3|$6|INCRBY|$3|foo|$1|1|]
11::46::44 trce: Session[0] [172.28.0.2:47706] [037A9C05] SEND: [:1|]
11::46::44 trce: Session[0] [172.28.0.2:47706] [037A9C05] RECV: [*3|$6|EXPIRE|$3|foo|$1|5|]
11::46::44 trce: Session[0] [172.28.0.2:47706] [037A9C05] SEND: [:1|]
11::46::46 trce: Session[0] [172.28.0.2:47706] [037A9C05] RECV: [*2|$3|GET|$3|foo|]
11::46::46 trce: Session[0] [172.28.0.2:47706] [037A9C05] SEND: [$1|1|]
11::46::51 trce: Session[0] [172.28.0.2:47706] [037A9C05] RECV: [*2|$3|GET|$3|foo|]
11::46::51 trce: Session[0] [172.28.0.2:47706] [037A9C05] SEND: [$-1|]
11::46::51 trce: Session[0] [172.28.0.2:47706] [037A9C05] RECV: [*3|$6|INCRBY|$3|foo|$1|1|]
11::46::51 trce: Session[0] [172.28.0.2:47706] [037A9C05] SEND: [:1W�+1|]
11::46::51 dbug: Session[0] [172.28.0.2:47706] [037A9C05] Disposing RespServerSession Id=1
```
