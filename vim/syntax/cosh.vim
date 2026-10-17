" Vim syntax file
" Language:		cosh
" Maintainer:	        Tom Harrison <tomhrr@tomhrr.org>
" Last Change:		2026 Oct 15
" Filenames:		*.ch
" URL:			https://github.com/tomhrr/cosh

" Adapted from https://github.com/jkotlinski/forth.vim, maintained
" by Johan Kotlinski.

if exists("b:current_syntax")
    finish
endif

let s:cpo_save = &cpo
set cpo&vim

syn case ignore

syn iskeyword 33-126,128-255

syn keyword coshOperators %;
syn keyword coshOperators &;
syn keyword coshOperators *;
syn keyword coshOperators **;
syn keyword coshOperators +;
syn keyword coshOperators ++;
syn keyword coshOperators +time;
syn keyword coshOperators -;
syn keyword coshOperators -time;
syn keyword coshOperators .ss;
syn keyword coshOperators /;
syn keyword coshOperators <;
syn keyword coshOperators <<;
syn keyword coshOperators <=>;
syn keyword coshOperators =;
syn keyword coshOperators >;
syn keyword coshOperators >>;
syn keyword coshOperators ^;
syn keyword coshOperators abs;
syn keyword coshOperators bigint;
syn keyword coshOperators bool;
syn keyword coshOperators byte;
syn keyword coshOperators c;
syn keyword coshOperators cd;
syn keyword coshOperators chmod;
syn keyword coshOperators chown;
syn keyword coshOperators chr;
syn keyword coshOperators clear;
syn keyword coshOperators clone;
syn keyword coshOperators close;
syn keyword coshOperators cmd;
syn keyword coshOperators cmd/c;
syn keyword coshOperators cmd/e;
syn keyword coshOperators cmd/eo;
syn keyword coshOperators cmd/o;
syn keyword coshOperators cmd/oe;
syn keyword coshOperators cp;
syn keyword coshOperators cwd;
syn keyword coshOperators date;
syn keyword coshOperators db.conn;
syn keyword coshOperators db.exec;
syn keyword coshOperators db.prep;
syn keyword coshOperators delete;
syn keyword coshOperators depth;
syn keyword coshOperators diff;
syn keyword coshOperators dns;
syn keyword coshOperators dns._nameservers;
syn keyword coshOperators dns.refresh;
syn keyword coshOperators dnsat;
syn keyword coshOperators drop;
syn keyword coshOperators dup;
syn keyword coshOperators each;
syn keyword coshOperators empty;
syn keyword coshOperators env;
syn keyword coshOperators exec;
syn keyword coshOperators exists;
syn keyword coshOperators exit;
syn keyword coshOperators expand-tilde;
syn keyword coshOperators float;
syn keyword coshOperators fmt;
syn keyword coshOperators fmtq;
syn keyword coshOperators from-epoch;
syn keyword coshOperators from-json;
syn keyword coshOperators from-xml;
syn keyword coshOperators from-yaml;
syn keyword coshOperators get;
syn keyword coshOperators getenv;
syn keyword coshOperators geteuid;
syn keyword coshOperators getpwuid;
syn keyword coshOperators getuid;
syn keyword coshOperators hex;
syn keyword coshOperators history;
syn keyword coshOperators hostname;
syn keyword coshOperators http;
syn keyword coshOperators http.get;
syn keyword coshOperators ifconfig;
syn keyword coshOperators int;
syn keyword coshOperators ip;
syn keyword coshOperators ip.addr;
syn keyword coshOperators ip.addr-int;
syn keyword coshOperators ip.from-int;
syn keyword coshOperators ip.last-addr;
syn keyword coshOperators ip.last-addr-int;
syn keyword coshOperators ip.len;
syn keyword coshOperators ip.prefixes;
syn keyword coshOperators ip.size;
syn keyword coshOperators ip.version;
syn keyword coshOperators ips;
syn keyword coshOperators is-bigint;
syn keyword coshOperators is-bool;
syn keyword coshOperators is-byte;
syn keyword coshOperators is-callable;
syn keyword coshOperators is-dir;
syn keyword coshOperators is-file;
syn keyword coshOperators is-float;
syn keyword coshOperators is-hash;
syn keyword coshOperators is-int;
syn keyword coshOperators is-link;
syn keyword coshOperators is-list;
syn keyword coshOperators is-null;
syn keyword coshOperators is-r;
syn keyword coshOperators is-set;
syn keyword coshOperators is-shiftable;
syn keyword coshOperators is-str;
syn keyword coshOperators is-w;
syn keyword coshOperators is-x;
syn keyword coshOperators isect;
syn keyword coshOperators jobs;
syn keyword coshOperators join;
syn keyword coshOperators keys;
syn keyword coshOperators kill;
syn keyword coshOperators last;
syn keyword coshOperators lc;
syn keyword coshOperators lcfirst;
syn keyword coshOperators len;
syn keyword coshOperators link;
syn keyword coshOperators lstat;
syn keyword coshOperators m;
syn keyword coshOperators md5;
syn keyword coshOperators mkdir;
syn keyword coshOperators mv;
syn keyword coshOperators nb;
syn keyword coshOperators nc;
syn keyword coshOperators netstat;
syn keyword coshOperators now;
syn keyword coshOperators oct;
syn keyword coshOperators open;
syn keyword coshOperators opendir;
syn keyword coshOperators ord;
syn keyword coshOperators over;
syn keyword coshOperators pmap;
syn keyword coshOperators pmapn;
syn keyword coshOperators pop;
syn keyword coshOperators print;
syn keyword coshOperators println;
syn keyword coshOperators ps;
syn keyword coshOperators pss;
syn keyword coshOperators push;
syn keyword coshOperators r;
syn keyword coshOperators rand;
syn keyword coshOperators read;
syn keyword coshOperators readdir;
syn keyword coshOperators readline;
syn keyword coshOperators readlink;
syn keyword coshOperators rename;
syn keyword coshOperators reverse;
syn keyword coshOperators rm;
syn keyword coshOperators rmdir;
syn keyword coshOperators rmf;
syn keyword coshOperators rmrf;
syn keyword coshOperators rot;
syn keyword coshOperators s;
syn keyword coshOperators set;
syn keyword coshOperators set-tz;
syn keyword coshOperators setenv;
syn keyword coshOperators sha1;
syn keyword coshOperators sha256;
syn keyword coshOperators sha512;
syn keyword coshOperators shift;
syn keyword coshOperators sleep;
syn keyword coshOperators socket;
syn keyword coshOperators sort;
syn keyword coshOperators sortp;
syn keyword coshOperators source;
syn keyword coshOperators split;
syn keyword coshOperators splitr;
syn keyword coshOperators sqrt;
syn keyword coshOperators stat;
syn keyword coshOperators status;
syn keyword coshOperators str;
syn keyword coshOperators strftime;
syn keyword coshOperators strptime;
syn keyword coshOperators strptimez;
syn keyword coshOperators style;
syn keyword coshOperators swap;
syn keyword coshOperators symdiff;
syn keyword coshOperators tempdir;
syn keyword coshOperators tempfile;
syn keyword coshOperators to-epoch;
syn keyword coshOperators to-json;
syn keyword coshOperators to-xml;
syn keyword coshOperators to-yaml;
syn keyword coshOperators touch;
syn keyword coshOperators uc;
syn keyword coshOperators ucfirst;
syn keyword coshOperators unhex;
syn keyword coshOperators union;
syn keyword coshOperators unoct;
syn keyword coshOperators unshift;
syn keyword coshOperators values;
syn keyword coshOperators write;
syn keyword coshOperators writeline;
syn keyword coshOperators \|;
syn keyword coshOperators \|\|;

syn keyword coshOperators var;
syn keyword coshOperators var!;
syn keyword coshOperators @;
syn keyword coshOperators !;
syn keyword coshOperators error;
syn keyword coshOperators yield;
syn keyword coshOperators to-function;
syn keyword coshOperators return;

syn keyword coshRT 2over;
syn keyword coshRT 2rot;
syn keyword coshRT <=;
syn keyword coshRT >=;
syn keyword coshRT after;
syn keyword coshRT afteri;
syn keyword coshRT all;
syn keyword coshRT and;
syn keyword coshRT any;
syn keyword coshRT before;
syn keyword coshRT beforei;
syn keyword coshRT chomp;
syn keyword coshRT f<;
syn keyword coshRT f>;
syn keyword coshRT first;
syn keyword coshRT flatten;
syn keyword coshRT foldl;
syn keyword coshRT for;
syn keyword coshRT funcall;
syn keyword coshRT grep;
syn keyword coshRT grep-generator;
syn keyword coshRT grep-list;
syn keyword coshRT id;
syn keyword coshRT is-list-or-set;
syn keyword coshRT ls;
syn keyword coshRT lsh;
syn keyword coshRT lshr;
syn keyword coshRT lsr;
syn keyword coshRT map;
syn keyword coshRT map-generator;
syn keyword coshRT map-list;
syn keyword coshRT max;
syn keyword coshRT min;
syn keyword coshRT nip;
syn keyword coshRT no-upwards;
syn keyword coshRT none;
syn keyword coshRT not;
syn keyword coshRT notall;
syn keyword coshRT or;
syn keyword coshRT pairwise;
syn keyword coshRT product;
syn keyword coshRT range;
syn keyword coshRT shuffle;
syn keyword coshRT slide;
syn keyword coshRT sum;
syn keyword coshRT take;
syn keyword coshRT take-all;
syn keyword coshRT uniq;

syn keyword coshCond else; if; then;

syn keyword coshLoop begin; until; leave;

syn match   coshColonDef "\<:\~\?\s*[^ \t]\+\>"
syn keyword coshEndOfColonDef ;
syn keyword coshEndOfDef ,,

syn match coshBoolean "\<\(.t\|.f\)\>"

syn match coshInteger '\<-\=\d\+\>'

syn match coshFunction "\<\S\+;\>"

syn match coshString "\<[^ ;]\+\>"
syn region coshStringSQ start=/'/ skip=/\\'/ end=/'/ oneline
syn region coshStringDQ start=/"/ skip=/\\"/ end=/"/ oneline

syn match coshComment "\v^\s*#.*$"

syn match coshFloat '\<\d\+\.\d\+\>'

syn region coshList start=/(/ end=/)/ contains=TOP
syn region coshSet start=/s(/ end=/)/ contains=TOP
syn region coshHash start=/h(/ end=/)/ contains=TOP
syn region coshCommand start=/{/ end=/}/ contains=TOP
syn region coshAnon start=/\[/ end=/\]/ contains=TOP

hi def link coshOperators Operator
hi def link coshRT Function
hi def link coshCond Conditional
hi def link coshLoop Repeat
hi def link coshBoolean Boolean
hi def link coshColonDef Define
hi def link coshEndOfColonDef Define
hi def link coshEndOfDef Define
hi def link coshInteger Number
hi def link coshString String
hi def link coshStringSQ String
hi def link coshStringDQ String
hi def link coshComment Comment
hi def link coshFloat Float
hi def link coshFunction Function
hi def link coshList Special
hi def link coshSet Special
hi def link coshHash Special
hi def link coshCommand Special
hi def link coshAnon Special

let b:current_syntax = "cosh"

let &cpo = s:cpo_save
unlet s:cpo_save
