Cobweb is a screen scraper, this is very early code, more a proof of concept, but as
with all the very best proofs of concept, it still provides some value.

The tool targets the Massachusetts Department of Health web site, which handles reservations for
obtaining vaccination against Covid-19, the application does nothing https://www.maimmunizations.org/clinic/search cannot do, but it does avoid paging through results and is a LOT quicker than querying by hand

Just run from the command line, with no parameters to see a list of clinics that advertise availability and if available, a link for reservation...

For example:

    ~\dev\cobweb [main +5 ~0 -0 !]> .\target\release\cobweb.exe
    Worcester Senior Center on 02/22/2021 has 40 available
    Gillette Stadium on 02/25/2021 has 1 available https://www.maimmunizations.org/client/registration?clinic_id=988
    Read 5 pages and found 44 clinics, of which 2 have availability

The scraper checked 5 pages to collate results, found 44 "clinics" listed, of which only two claimed to have availability, and only Gillette provided a link for registration.

You can list all the clinics found, including those with no availability using the -a option, or list from some future date onwards, using the -f option.

To list all the options, run with --help

    ~\dev\cobweb [main +5 ~0 -0 !]> .\target\release\cobweb.exe --help
    Usage: cobweb.exe [-a] [-f <from>]

    Cobweb, a CLI for PrepMod

    Options:
    -a, --all         show all clinics, even those with no availability
    -f, --from        start search from a date, for example, -f 2021-02-25
    --help            display usage information
