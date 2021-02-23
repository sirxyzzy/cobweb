Cobweb is a command line oriented screen scraper, this is very early code, more a proof of concept,
but as with all the very best proofs of concept, it still provides some value.

I was inspired by the awesome pages at https://www.macovidvaccines.com/, which provide a much
more glossy and user friendly way of presenting this data.

The tool targets the Massachusetts Department of Health web site, which handles reservations for
obtaining vaccination against Covid-19. Cobweb does nothing that the official 
https://www.maimmunizations.org/clinic/search cannot do, but it does avoid paging
through results so is a lot quicker than querying by hand.

## Disclaimer

This program is not affiliated with or endorsed by the Commonwealth of Massachusetts.
The information may not be complete or accurate. It may break without prior warning,
it may offend those of a sensitive disposition, or curdle fresh milk, whether it works at
all depends on the underlying web site, and remember, it's only code!

## Using Cobweb

Run from the command line, with no parameters to see a list of clinics that advertise availability and if available, a link for reservation...

For example:

    > cobweb
    Searching https://www.maimmunizations.org
    Worcester Senior Center on 02/22/2021 has 41 available
    Tree House Deerfield on 02/25/2021 has 1 available
    Register at https://www.maimmunizations.org/client/registration?clinic_id=2040

    Tree House Deerfield on 02/26/2021 has 1 available
    Register at https://www.maimmunizations.org/client/registration?clinic_id=2041

    Read 5 pages and found 44 clinics, of which 3 have availability

The scraper saves time by checking multiple pages to collate the results

You can list all the clinics found, even those with no availability using the -a option, or list from some future date onwards, using the -f option, which can be useful as no more than 50 results can come back at
any one time.

To list all the options, run with the --help flag

