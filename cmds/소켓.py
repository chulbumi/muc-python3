# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
            
        msg = ''
        slist = []
        for ply in ob.channel.players:
            try:
                slist.append( (ply.channel.transport.getPeer().host, ply['이름']) ) 
            except:
                continue
        slist.sort()
        old = ''
        for l in slist:
           if l[1] == '':
               continue
           if old != l[0]:
               msg += '\r\n%16s : %s' % (l[0], l[1])
           else:
               msg += ', %s' % l[1]
           old = l[0]
        ob.sendLine(msg)
