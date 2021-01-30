# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        msg = '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n'
        msg += '☆ 방파 리스트\r\n'
        msg += '───────────────────────────────────────\r\n'
        for g in GUILD.attr:
            guild = GUILD.attr[g]
            buf = '[%s]' % guild['이름']
            if  getInt(ob['관리자등급']) >= 1000:
                msg += '%-12s : %-30s   %3d 명 %s\r\n' % (buf, guild['방주이름'], guild['방파원수'], guild['방파맵'])
            else:
                msg += '%-12s : %-30s   %3d 명\r\n' % (buf, guild['방주이름'], guild['방파원수'])

        msg += '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━'
        ob.sendLine(msg)
