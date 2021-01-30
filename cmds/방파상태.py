# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if ob['소속'] == '':
            ob.sendLine('☞ 당신은 소속이 없습니다.')
            return
        g = GUILD[ob['소속']]
        l1 = []
        l2 = []
        l3 = []
        if '부방주리스트' in g:
            l1 = g['부방주리스트']
        if '장로리스트' in g:
            l2 = g['장로리스트']
        if '방파인리스트' in g:
            l3 = g['방파인리스트']

        Num = 0
        msg = MAIN_CONFIG['방파상태출력상단']
        msg += MAIN_CONFIG['방파상태출력']
        msg += '[1;31m[1;47m%s[0;37;40m\r\n' % ob['소속']
        msg += MAIN_CONFIG['방파상태출력하단']
        msg += '\r\n  [[1m[31m방  주[0m[40m[37m]     %-11s' % g['방주이름']
        Num += 1
        for buf in l1:
            msg += '  [[1m[33m부방주[0m[40m[37m]     %-11s' % buf
            Num += 1
        if Num % 3 == 0:
            msg += '\r\n'
        for buf in l2:
            msg += '  [[1m[32m장  로[0m[40m[37m]     %-11s' % buf
            Num += 1
            if Num % 3 == 0:
                msg += '\r\n'
        for buf in l3:
            msg += '  [[1m방파원[0m[40m[37m]     %-11s' % buf
            Num += 1
            if Num % 3 == 0:
                msg += '\r\n'

        msg += '\r\n' + MAIN_CONFIG['방파상태출력하단']
        msg += '\r\n방파총인원 : %-8d' % g['방파원수']
        cnt = 0
        for ply in ob.channel.players:
            if ply['소속'] == ob['소속'] and ply.state == ACTIVE and getInt(ply['투명상태']) != 1:
                cnt += 1
        msg += '☞ 현재 %d명이 활동중 입니다.' % cnt
        
        ob.sendLine(msg)
        
