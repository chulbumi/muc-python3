# -*- coding: utf-8 -*-

from objs.cmd import Command
from lib.func import fillSpace

class CmdObj(Command):

    def cmd(self, ob, line):
        if line != '' and getInt(ob['관리자등급']) >= 1000:
            target = ob.env.findObjName(line)
            if target == None or is_item(target):
                ob.sendLine('☞ 당신의 안광으로는 그런것을 볼수 없다네')
                return
        else:
            target = ob
        
        ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
        if target == ob:
            buf = '◁ 당신의 무공 ▷'
        else:
            buf = '◁ %s의 무공 ▷' % target['이름']
        ob.sendLine('[0m[47m[30m%-71s[0m[40m[37m' % buf)
        ob.sendLine('───────────────────────────────────────')
        #ob.sendLine('[1m[40m[32m▷ 일반무공[0m[40m[37m')
        msg = ''
        if len(target.skillList) == 0:
            ob.sendLine('☞ 깨우친 무공이 없습니다.')
        else:
            for mname in target.skillLvType:
                mname += '무공'
                if len(MAIN_CONFIG[mname]) == 0:
                    continue
                msg += '[1m[40m[32m▷ %s[0m[40m[37m\r\n' % mname
                slist = MAIN_CONFIG[mname]
                c = 0
                #for mm in slist.split('\r\n'):
                for mm in slist:
                    m = mm.strip()
                    if m == '':
                        continue
                    #print(m)
                    if m not in target.skillMap:
                        if m in target.skillList:
                            s =1
                        else:
                            continue
                    else:
                        s = target.skillMap[m][0]
                    buf = '%s(%d성)' % (m, s)
                    msg += ' ◇ %s ' % fillSpace(20, buf)
                    c += 1
                    if c % 3 == 0:
                        msg += '\r\n'
                if c % 3 == 0:
                    msg = msg[:-2]
                msg += '\r\n'
                          
            msg = msg[:-2]
            ob.sendLine(msg)
        ob.sendLine('───────────────────────────────────────')
        
        ob.sendLine('[1m[40m[32m▷ 비전[0m[40m[37m')
        buf = target['비전수련']
        lines = target['비전이름']
        if buf == '' and len(lines) == 0:
            ob.sendLine('☞ 오의를 깨우친 무공이 없습니다.')
        else:
            if buf != '':
                msg = '[1m[33m%s[0m[40m[37m(수련중)\r\n' % buf
            else:
                msg = ''
            c = 0
            for m in lines:
                #msg += ' ◇ %-20s ' % m
                msg += ' ◇ %s ' % fillSpace(20, m)
                c += 1
                if c % 3 == 0:
                    msg += '\r\n'
            if c % 3 == 0:
                msg = msg[:-2]
            ob.sendLine(msg)
        ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
        
