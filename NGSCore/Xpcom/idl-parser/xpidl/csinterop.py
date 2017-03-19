#!/usr/bin/env python
# csinterop.py - Generate C# interop codes from IDL.
#
# Requires Python 2.x.
# Based on xpidl2cs.pl and header.py

import sys, os.path, re, xpidl, json

header = """/*
 * DO NOT EDIT. THIS FILE WAS GENERATED BY csinterop.py
 */
using System;
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;
using System.Text;
"""

cs_config = None

def print_cs(idl, fd, filename):
    fd.write(header)
    fd.write('\n')
    fd.write("namespace %s {\n" % (cs_config['namespace']))

    for p in idl.productions:
        if p.kind == 'interface':
            write_interface(p, fd)

    fd.write("}\n")

builtin_map = {
    'boolean': 'bool',
    'void': 'void',
    'octet': 'byte',
    'short': 'short',
    'long': 'int',
    'long long': 'long',
    'unsigned short': 'ushort',
    'unsigned long': 'uint',
    'unsigned long long': 'ulong',
    'float': 'float',
    'double': 'double',
    'char': 'sbyte',
    'string': '[MarshalAs(UnmanagedType.LPStr)] string',
    # 'wchar': '',
    'wstring': '[MarshalAs(UnmanagedType.LPWStr)] string',
}

def write_interface(iface, fd):
    def type_as_cs(t):
        if isinstance(t, xpidl.Interface):
            return '[MarshalAs(UnmanagedType.Interface)] ' + t.name
        elif isinstance(t, xpidl.Builtin):
            return builtin_map[t.name]
        elif isinstance(t, xpidl.Native) or isinstance(t, xpidl.Typedef):
            native_map = cs_config['typeMappings']
            if t.name in native_map:
                return native_map[t.name]

        raise Exception("Cannot map the type %s" % t)

    # [hoge] piyo --> [target: hoge] piyo
    def attr_explicit_target(target, cs_type):
        if cs_type[0] == '[':
            return '[' + target + ': ' + cs_type[1:]
        else:
            return cs_type

    # [hoge] piyo --> [return: hoge] piyo
    def attr_to_return_value(cs_type):
        return attr_explicit_target('return', cs_type)

    # [hoge] piyo --> [hoge]
    #        piyo --> (empty)
    def leave_only_attr(cs_type):
        if cs_type[0] == '[':
            assert ']' in cs_type
            return cs_type[0:cs_type.find(']') + 1]
        else:
            return ''

    # [hoge] piyo --> piyo
    #        piyo --> piyo
    def remove_attr(cs_type):
        if cs_type[0] == '[':
            assert ']' in cs_type
            return cs_type[cs_type.find(']') + 1:].strip()
        else:
            return cs_type

    def get_return_type_as_cs(m):
        params = m.params
        if len(params) > 0 and params[-1].retval:
            return type_as_cs(params[-1].realtype)
        else:
            return type_as_cs(m.realtype)

    def get_param_as_cs(m):
        if m.paramtype == 'in':
            return '%s %s' % (type_as_cs(m.realtype), m.name)
        else:
            return '%s %s %s' % (m.paramtype, type_as_cs(m.realtype), m.name)

    def get_param_list_as_cs(m):
        params = m.params
        if len(params) > 0 and params[-1].retval:
            params = params[:-1]
        return ', '.join([get_param_as_cs(p) for p in params])

    def write_method_decl(m):
        fd.write('\t\t/* %s */\n' % (m.toIDL()))
        fd.write('\t\t[MethodImpl(MethodImplOptions.InternalCall, MethodCodeType = MethodCodeType.Runtime)]\n')
        fd.write('\t\t%s %s(%s);\n\n' % (attr_to_return_value(get_return_type_as_cs(m)), m.name,
            get_param_list_as_cs(m)))

    def write_attr_decl(a):
        fd.write('\t\t/* %s */\n' % (a.toIDL()))
        cs_type = type_as_cs(a.realtype)
        fd.write('\t\t%s %s\n' % (remove_attr(cs_type), a.name))
        fd.write('\t\t{\n')
        fd.write('\t\t\t[MethodImpl(MethodImplOptions.InternalCall, MethodCodeType = MethodCodeType.Runtime)]\n')
        fd.write('\t\t\t%s\n' % (leave_only_attr(attr_explicit_target('return', cs_type))))
        fd.write('\t\t\tget;\n')
        if not a.readonly:
            fd.write('\t\t\t[MethodImpl(MethodImplOptions.InternalCall, MethodCodeType = MethodCodeType.Runtime)]\n')
            fd.write('\t\t\t%s\n' % (leave_only_attr(attr_explicit_target('param', cs_type))))
            fd.write('\t\t\tset;\n')
        fd.write('\t\t}\n')

    if iface.namemap is None:
        raise Exception("Interface was not resolved.")

    fd.write('\t[Guid("%s")]\n' % (iface.attributes.uuid))
    fd.write('\t[InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]\n')
    fd.write('\t[ComImport()]\n')
    if iface.base and iface.base != 'nsISupports':
        fd.write('\tpublic interface %s : %s\n' % (iface.name, iface.base))
    else:
        fd.write('\tpublic interface %s\n' % (iface.name))
    fd.write('\t{\n')

    for member in iface.members:
        if isinstance(member, xpidl.Attribute):
            write_attr_decl(member)
        elif isinstance(member, xpidl.Method):
            write_method_decl(member)
        elif isinstance(member, xpidl.CDATA):
            pass
        else:
            raise Exception("Unexpected interface member: %s" % member)
    fd.write('\t}\n\n')


if __name__ == '__main__':
    from optparse import OptionParser
    o = OptionParser()
    o.add_option('-I', action='append', dest='incdirs', default=['.'],
                 help="Directory to search for imported files")
    o.add_option('--cachedir', dest='cachedir', default=None,
                 help="Directory in which to cache lex/parse tables.")
    o.add_option('-o', dest='outfile', default=None,
                 help="Output file (default is stdout)")
    o.add_option('-c', dest='configfile', default=None,
                 help="JSON config file")
    o.add_option('--regen', action='store_true', dest='regen', default=False,
                 help="Regenerate IDL Parser cache")
    options, args = o.parse_args()
    file = args[0] if args else None

    if options.cachedir is not None:
        if not os.path.isdir(options.cachedir):
            os.mkdir(options.cachedir)
        sys.path.append(options.cachedir)
    else:
        print >>sys.stderr, "--cachedir is mandatory"
        sys.exit(1)

    if options.regen:
        if options.cachedir is None:
            print >>sys.stderr, "--regen requires --cachedir"
            sys.exit(1)

        p = xpidl.IDLParser(outputdir=options.cachedir, regen=True)
        sys.exit(0)

    if options.configfile is not None:
        with open(options.configfile) as f:
            cs_config = json.load(f)
    else:
        print >>sys.stderr, "-c is mandatory"
        sys.exit(1)

    if options.outfile is not None:
        outfd = open(options.outfile, 'w')
        closeoutfd = True
    else:
        outfd = sys.stdout
        closeoutfd = False

    p = xpidl.IDLParser(outputdir=options.cachedir)
    idl = p.parse(open(file).read(), filename=file)
    idl.resolve(options.incdirs, p)

    print_cs(idl, outfd, file)

    if closeoutfd:
        outfd.close()