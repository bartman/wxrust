- general structure of command line
    ```
    wxrust [...global options] <cmd> [...command options]
    ```

- show help:
    - general help about tool (no command details)
        ```
        wxrust -h
        wxrust help
        ```

    - command details
        ```
        wxrust <cmd> -h
        wxrust <cmd> help
        ```

- global options
    - change credentials file location
        ```
        wxrust -c credentials_path <cmd> ...
        wxrust --credentials credentials_path <cmd> ...
        ```

    - force login, ignore cached auth token
        ```
        wxrust -a --force-authentication <cmd> ...
        ```

- listing workouts

    - general format of command
        ```
        wxrust list [-d|--details] [-s|--summary] [-A|--all]
        wxrust list [-d|--details] [-s|--summary] [-c|--count] <count>
        wxrust list [-d|--details] [-s|--summary] [-y|--year] <YYYY>
        wxrust list [-d|--details] [-s|--summary] [-m|--month] <YYYYMM> ...
        wxrust list [-d|--details] [-s|--summary] [-b|--before] <end-date> [-c|--count] <count>
        wxrust list [-d|--details] [-s|--summary] <range-start>-<range-end>
        ```

    - list all dates of workouts
        ```
        wxrust list
        wxrust list -A
        wxrust list --all
        ```

    - the most recent workout dates
        ```
        wxrust list --count 10
        wxrust list -c 1
        ```

    - restrict to some year(s)
        ```
        wxrust list -y 2025
        wxrust list --year 2025
        wxrust list -y 2024 2025
        wxrust list --year 2025 2024
        ```

    - restrict to some month(s)
        ```
        wxrust list --month 2025.10 2025.11
        wxrust list -m 2025/10 2025/11
        wxrust list --month 202510 202511
        ```

    - restrict range to year range (all workouts in range of years inclusive)
        ```
        wxrust list 2024-2025
        ```
        (this is equivalent to 20240101-20251231)

    - restrict range to month range (all workouts in range of months inclusive)
        ```
        wxrust list 2025.10-2025.11
        wxrust list 2025/10-2025/11
        wxrust list 202510-202511
        ```
        (this is equivalent to 20251001-20251130)

    - restrict range to specific date range (all workouts in range of months inclusive)
        ```
        wxrust list 2025.10.10-2025.11.01
        wxrust list 2025/10/10-2025/11/01
        wxrust list 20251010-20251101
        ```

    - by default show only the dates of workouts (no details or summary), using YYYY-MM-DD

    - show details of each workout listed (full workout details)
        ```
        wxrust list -d ...
        wxrust list --details ...
        ```

    - show summary of each workout (one line summary of workout)
        ```
        wxrust list -s ...
        wxrust list --summary ...
        ```
        (a summary is a list of exercises, separated with semicolon, name + heaviest set for each: deadlift 515x3)


- showing workout details

    - general format of command
        ```
        wxrust show [-s|--summary] <date>
        ```

    - showing one day

    ```
    wxrust show <date>
    ```

    - showing only summary

    ```
    wxrust show --summary <date>
    ```


