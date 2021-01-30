# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if ob['소속'] == '':
            ob.sendLine('☞ 당신은 소속이 없습니다.')
            return
        if line == '':
            ob.sendLine('☞ 사용법 : [내용] 방파말(])')
            return
        if ob.checkConfig('방파말거부'):
            ob.sendLine('☞ 방파말 거부중 이에요. *^^*')
            return
        try:    
            fp = open('data/log/group/' + ob['소속'], 'a')
        except:
            pass
        fp.write(time.strftime('[%Y-%m-%d %H:%M:%S] ', time.localtime()) + '%-10s' % ob['이름'] + ': ' + line + '\n')
        fp.close()
        ob.sendGroup(line, prompt = True)
        
